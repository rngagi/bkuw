use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use fs2::FileExt;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Transaction, backup::Backup, params};
use uuid::Uuid;

use crate::{
    domain::{
        CreateProjectRequest, DeleteEntryRequest, DeletedEntry, EntryForm, EntryRelation,
        EntrySummary, Example, ExampleForm, LexicalEntry, Project, ProjectSnapshot,
        SaveEntryRequest, Sense, UpdateProjectSettingsRequest, WritingSystem,
    },
    error::{AppError, AppResult},
    search::{normalize_text, search_key},
};

const INITIAL_MIGRATION: &str = include_str!("../migrations/001_initial.sql");
const METADATA_OPTIONS_MIGRATION: &str = include_str!("../migrations/002_metadata_options.sql");
const LATEST_SCHEMA_VERSION: i64 = 2;

pub struct ProjectSession {
    root: PathBuf,
    connection: Connection,
    lock_file: File,
}

impl ProjectSession {
    pub fn create(request: CreateProjectRequest) -> AppResult<Self> {
        validate_project_name(&request.name)?;
        let parent = PathBuf::from(request.parent_dir)
            .canonicalize()
            .map_err(|error| {
                AppError::with_details(
                    "invalid_project",
                    "The selected parent folder is not available.",
                    error.to_string(),
                )
            })?;
        if !parent.is_dir() {
            return Err(AppError::new(
                "invalid_project",
                "The selected parent path is not a folder.",
            ));
        }

        let root = parent.join(format!("{}.bkuw", request.name.trim()));
        if root.exists() {
            return Err(AppError::new(
                "project_exists",
                "A project with this name already exists in the selected folder.",
            ));
        }

        fs::create_dir(&root)?;
        let result = (|| {
            fs::create_dir(root.join("backups"))?;
            let lock_file = acquire_lock(&root)?;
            let database_path = root.join("project.sqlite");
            let mut connection = connect(&database_path)?;
            migrate(&mut connection, &root)?;

            let project_id = new_id();
            let writing_system_id = new_id();
            let now = now();
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO projects
                 (id, name, language_name, language_code, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)",
                params![
                    project_id,
                    normalize_text(request.name.trim()),
                    normalize_optional(request.language_name),
                    normalize_optional(request.language_code),
                    now
                ],
            )?;
            transaction.execute(
                "INSERT INTO writing_systems
                 (id, project_id, name, type, display_role, sort_order)
                 VALUES (?1, ?2, 'Primary orthography', 'orthography', 'primary', 0)",
                params![writing_system_id, project_id],
            )?;
            transaction.commit()?;

            Ok(Self {
                root: root.clone(),
                connection,
                lock_file,
            })
        })();

        if result.is_err() {
            let _ = fs::remove_dir_all(&root);
        }
        result
    }

    pub fn open(path: impl AsRef<Path>) -> AppResult<Self> {
        let root = path.as_ref().canonicalize().map_err(|error| {
            AppError::with_details(
                "invalid_project",
                "The selected project folder is not available.",
                error.to_string(),
            )
        })?;
        let has_project_extension = root
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("bkuw"));
        if !root.is_dir() || !has_project_extension {
            return Err(AppError::new(
                "invalid_project",
                "Select a .bkuw project folder.",
            ));
        }

        let database_path = root.join("project.sqlite");
        if !database_path.is_file() {
            return Err(AppError::new(
                "invalid_project",
                "The selected folder does not contain project.sqlite.",
            ));
        }

        let lock_file = acquire_lock(&root)?;
        validate_database_identity(&database_path)?;
        let mut connection = connect(&database_path)?;
        migrate(&mut connection, &root)?;
        let project_count: i64 =
            connection.query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))?;
        if project_count != 1 {
            return Err(AppError::new(
                "invalid_project",
                "The database does not contain exactly one bkuw project.",
            ));
        }

        Ok(Self {
            root,
            connection,
            lock_file,
        })
    }

    pub fn close(self) -> AppResult<()> {
        FileExt::unlock(&self.lock_file)?;
        Ok(())
    }

    pub fn snapshot(&self) -> AppResult<ProjectSnapshot> {
        Ok(ProjectSnapshot {
            root_path: self.root.to_string_lossy().into_owned(),
            project: load_project(&self.connection)?,
            writing_systems: load_writing_systems(&self.connection)?,
            part_of_speech_options: load_metadata_options(&self.connection, "part_of_speech")?,
            semantic_domain_options: load_metadata_options(&self.connection, "semantic_domain")?,
            entries: query_summaries(&self.connection, "")?,
        })
    }

    pub fn update_settings(
        &mut self,
        request: UpdateProjectSettingsRequest,
    ) -> AppResult<ProjectSnapshot> {
        validate_writing_systems(&request.writing_systems)?;
        validate_metadata_options(&request.part_of_speech_options)?;
        validate_metadata_options(&request.semantic_domain_options)?;
        if request.name.trim().is_empty() {
            return Err(AppError::new("validation", "Project name is required."));
        }
        let project_id = project_id(&self.connection)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE projects
             SET name = ?1, language_name = ?2, language_code = ?3,
                 description = ?4, updated_at = ?5
             WHERE id = ?6",
            params![
                normalize_text(request.name.trim()),
                normalize_optional(request.language_name),
                normalize_optional(request.language_code),
                normalize_optional(request.description),
                now(),
                project_id
            ],
        )?;

        transaction.execute(
            "UPDATE writing_systems SET display_role = NULL WHERE project_id = ?1",
            params![project_id],
        )?;

        for writing_system in &request.writing_systems {
            transaction.execute(
                "INSERT INTO writing_systems
                 (id, project_id, name, type, script_code, language_tag, display_role,
                  sort_order, font_family, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   type = excluded.type,
                   script_code = excluded.script_code,
                   language_tag = excluded.language_tag,
                   display_role = excluded.display_role,
                   sort_order = excluded.sort_order,
                   font_family = excluded.font_family,
                   notes = excluded.notes
                 WHERE writing_systems.project_id = excluded.project_id",
                params![
                    writing_system.id,
                    project_id,
                    normalize_text(writing_system.name.trim()),
                    writing_system.kind,
                    normalize_optional(writing_system.script_code.clone()),
                    normalize_optional(writing_system.language_tag.clone()),
                    writing_system.display_role,
                    writing_system.sort_order,
                    normalize_optional(writing_system.font_family.clone()),
                    normalize_optional(writing_system.notes.clone())
                ],
            )?;
        }

        let incoming_ids = request
            .writing_systems
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>();
        let mut existing_statement =
            transaction.prepare("SELECT id FROM writing_systems WHERE project_id = ?1")?;
        let existing_ids = existing_statement
            .query_map(params![project_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(existing_statement);
        for existing_id in existing_ids {
            if !incoming_ids.contains(&existing_id.as_str()) {
                transaction.execute(
                    "DELETE FROM writing_systems WHERE id = ?1 AND project_id = ?2",
                    params![existing_id, project_id],
                )?;
            }
        }
        replace_metadata_options(
            &transaction,
            &project_id,
            "part_of_speech",
            &request.part_of_speech_options,
        )?;
        replace_metadata_options(
            &transaction,
            &project_id,
            "semantic_domain",
            &request.semantic_domain_options,
        )?;
        transaction.commit()?;
        self.snapshot()
    }

    pub fn query_entries(&self, query: &str) -> AppResult<Vec<EntrySummary>> {
        query_summaries(&self.connection, query)
    }

    pub fn load_entry(&self, id: &str) -> AppResult<LexicalEntry> {
        load_entry(&self.connection, id, false)
    }

    pub fn create_entry(&mut self) -> AppResult<LexicalEntry> {
        let project_id = project_id(&self.connection)?;
        let id = new_id();
        let timestamp = now();
        self.connection.execute(
            "INSERT INTO lexical_entries
             (id, project_id, notes, revision, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, NULL, 0, ?3, ?3, NULL)",
            params![id, project_id, timestamp],
        )?;
        load_entry(&self.connection, &id, false)
    }

    pub fn save_entry(&mut self, request: SaveEntryRequest) -> AppResult<LexicalEntry> {
        validate_entry(&request.entry)?;
        let id = request.entry.id.clone();
        let timestamp = now();
        let transaction = self.connection.transaction()?;
        let updated = transaction.execute(
            "UPDATE lexical_entries
             SET notes = ?1, revision = revision + 1, updated_at = ?2
             WHERE id = ?3 AND revision = ?4 AND deleted_at IS NULL",
            params![
                normalize_optional(request.entry.notes.clone()),
                timestamp,
                id,
                request.expected_revision
            ],
        )?;
        if updated == 0 {
            return Err(revision_or_not_found(&transaction, &id)?);
        }

        transaction.execute("DELETE FROM entry_forms WHERE entry_id = ?1", params![id])?;
        transaction.execute("DELETE FROM senses WHERE entry_id = ?1", params![id])?;
        transaction.execute(
            "DELETE FROM entry_relations WHERE source_entry_id = ?1",
            params![id],
        )?;

        insert_forms(&transaction, &id, &request.entry.forms)?;
        insert_senses(&transaction, &id, &request.entry.senses)?;
        insert_relations(&transaction, &id, &request.entry.relations)?;
        transaction.commit()?;
        load_entry(&self.connection, &id, false)
    }

    pub fn delete_entry(&mut self, request: DeleteEntryRequest) -> AppResult<DeletedEntry> {
        let deleted_at = now();
        let updated = self.connection.execute(
            "UPDATE lexical_entries
             SET deleted_at = ?1, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND revision = ?3 AND deleted_at IS NULL",
            params![deleted_at, request.id, request.expected_revision],
        )?;
        if updated == 0 {
            return Err(revision_or_not_found(&self.connection, &request.id)?);
        }
        Ok(DeletedEntry {
            id: request.id,
            deleted_at,
        })
    }

    pub fn restore_entry(&mut self, id: &str) -> AppResult<LexicalEntry> {
        let updated = self.connection.execute(
            "UPDATE lexical_entries
             SET deleted_at = NULL, updated_at = ?1, revision = revision + 1
             WHERE id = ?2 AND deleted_at IS NOT NULL",
            params![now(), id],
        )?;
        if updated == 0 {
            return Err(AppError::new(
                "not_found",
                "The deleted entry was not found.",
            ));
        }
        load_entry(&self.connection, id, false)
    }
}

fn connect(database_path: &Path) -> AppResult<Connection> {
    let connection = Connection::open(database_path)?;
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    Ok(connection)
}

fn validate_database_identity(database_path: &Path) -> AppResult<()> {
    let connection = Connection::open_with_flags(database_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| {
        AppError::with_details(
            "invalid_project",
            "The selected project database cannot be read.",
            error.to_string(),
        )
    })?;
    let required_tables: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type = 'table' AND name IN ('schema_migrations', 'projects')",
        [],
        |row| row.get(0),
    )?;
    if required_tables != 2 {
        return Err(AppError::new(
            "invalid_project",
            "The selected database is not a bkuw project.",
        ));
    }
    Ok(())
}

fn migrate(connection: &mut Connection, root: &Path) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version INTEGER PRIMARY KEY,
           applied_at TEXT NOT NULL
         );",
    )?;
    let current: i64 = connection.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;
    if current > LATEST_SCHEMA_VERSION {
        return Err(AppError::new(
            "unsupported_schema",
            "This project was created by a newer version of bkuw.",
        ));
    }
    if current == LATEST_SCHEMA_VERSION {
        return Ok(());
    }
    if current > 0 {
        create_backup(connection, root)?;
    }

    let transaction = connection.transaction()?;
    if current < 1 {
        transaction.execute_batch(INITIAL_MIGRATION)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (1, ?1)",
            params![now()],
        )?;
    }
    if current < 2 {
        transaction.execute_batch(METADATA_OPTIONS_MIGRATION)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (2, ?1)",
            params![now()],
        )?;
    }
    transaction.commit()?;
    Ok(())
}

fn create_backup(source: &Connection, root: &Path) -> AppResult<PathBuf> {
    let backups = root.join("backups");
    fs::create_dir_all(&backups)?;
    let path = backups.join(format!(
        "project-{}.sqlite",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    let mut destination = Connection::open(&path)?;
    let backup = Backup::new(source, &mut destination)?;
    backup.run_to_completion(5, Duration::from_millis(10), None)?;
    drop(backup);
    Ok(path)
}

fn acquire_lock(root: &Path) -> AppResult<File> {
    let lock_path = root.join(".bkuw.lock");
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(lock_path)?;
    file.try_lock_exclusive().map_err(|error| {
        AppError::with_details(
            "project_locked",
            "This project is already open in another bkuw process.",
            error.to_string(),
        )
    })?;
    Ok(file)
}

fn load_project(connection: &Connection) -> AppResult<Project> {
    connection
        .query_row(
            "SELECT id, name, language_name, language_code, description, created_at, updated_at
             FROM projects LIMIT 1",
            [],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    language_name: row.get(2)?,
                    language_code: row.get(3)?,
                    description: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .map_err(Into::into)
}

fn load_writing_systems(connection: &Connection) -> AppResult<Vec<WritingSystem>> {
    let mut statement = connection.prepare(
        "SELECT id, name, type, script_code, language_tag, display_role,
                sort_order, font_family, notes
         FROM writing_systems ORDER BY sort_order, name",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(WritingSystem {
            id: row.get(0)?,
            name: row.get(1)?,
            kind: row.get(2)?,
            script_code: row.get(3)?,
            language_tag: row.get(4)?,
            display_role: row.get(5)?,
            sort_order: row.get(6)?,
            font_family: row.get(7)?,
            notes: row.get(8)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_metadata_options(connection: &Connection, kind: &str) -> AppResult<Vec<String>> {
    let mut statement = connection.prepare(
        "SELECT value FROM metadata_options
         WHERE kind = ?1 ORDER BY sort_order, value",
    )?;
    let rows = statement.query_map(params![kind], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn replace_metadata_options(
    transaction: &Transaction<'_>,
    project_id: &str,
    kind: &str,
    values: &[String],
) -> AppResult<()> {
    transaction.execute(
        "DELETE FROM metadata_options WHERE project_id = ?1 AND kind = ?2",
        params![project_id, kind],
    )?;
    for (sort_order, value) in values.iter().enumerate() {
        transaction.execute(
            "INSERT INTO metadata_options(id, project_id, kind, value, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                new_id(),
                project_id,
                kind,
                normalize_text(value.trim()),
                sort_order
            ],
        )?;
    }
    Ok(())
}

fn query_summaries(connection: &Connection, query: &str) -> AppResult<Vec<EntrySummary>> {
    let key = search_key(query.trim());
    let pattern = format!("%{key}%");
    let mut statement = connection.prepare(
        "SELECT e.id,
                COALESCE((
                  SELECT f.text FROM entry_forms f
                  JOIN writing_systems w ON w.id = f.writing_system_id
                  WHERE f.entry_id = e.id AND w.display_role = 'primary'
                  ORDER BY f.sort_order LIMIT 1
                ), ''),
                (SELECT f.text FROM entry_forms f
                 JOIN writing_systems w ON w.id = f.writing_system_id
                 WHERE f.entry_id = e.id AND w.display_role = 'secondary'
                 ORDER BY f.sort_order LIMIT 1),
                COALESCE((SELECT group_concat(DISTINCT s.part_of_speech)
                          FROM senses s
                          WHERE s.entry_id = e.id AND trim(COALESCE(s.part_of_speech, '')) <> ''), ''),
                e.revision
         FROM lexical_entries e
         WHERE e.deleted_at IS NULL
           AND (?1 = '' OR EXISTS (
             SELECT 1 FROM entry_forms f
             WHERE f.entry_id = e.id AND f.search_key LIKE ?2
           ))
         ORDER BY 2 COLLATE NOCASE, e.created_at, e.id",
    )?;
    let rows = statement.query_map(params![key, pattern], |row| {
        let joined: String = row.get(3)?;
        Ok(EntrySummary {
            id: row.get(0)?,
            primary_form: row.get(1)?,
            secondary_form: row.get(2)?,
            parts_of_speech: joined
                .split(',')
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
            revision: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_entry(connection: &Connection, id: &str, include_deleted: bool) -> AppResult<LexicalEntry> {
    let entry = connection
        .query_row(
            "SELECT id, notes, revision, created_at, updated_at
             FROM lexical_entries
             WHERE id = ?1 AND (?2 OR deleted_at IS NULL)",
            params![id, include_deleted],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("not_found", "The lexical entry was not found."))?;

    Ok(LexicalEntry {
        id: entry.0,
        notes: entry.1,
        revision: entry.2,
        created_at: entry.3,
        updated_at: entry.4,
        forms: load_forms(connection, id)?,
        senses: load_senses(connection, id)?,
        relations: load_relations(connection, id)?,
    })
}

fn load_forms(connection: &Connection, entry_id: &str) -> AppResult<Vec<EntryForm>> {
    let mut statement = connection.prepare(
        "SELECT id, writing_system_id, text, variant_label, dialect, status, notes, sort_order
         FROM entry_forms WHERE entry_id = ?1 ORDER BY sort_order, id",
    )?;
    let rows = statement.query_map(params![entry_id], |row| {
        Ok(EntryForm {
            id: row.get(0)?,
            writing_system_id: row.get(1)?,
            text: row.get(2)?,
            variant_label: row.get(3)?,
            dialect: row.get(4)?,
            status: row.get(5)?,
            notes: row.get(6)?,
            sort_order: row.get(7)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_senses(connection: &Connection, entry_id: &str) -> AppResult<Vec<Sense>> {
    let mut statement = connection.prepare(
        "SELECT id, gloss, definition, part_of_speech, semantic_domain, sort_order
         FROM senses WHERE entry_id = ?1 ORDER BY sort_order, id",
    )?;
    let rows = statement.query_map(params![entry_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let raw = rows.collect::<Result<Vec<_>, _>>()?;
    raw.into_iter()
        .map(|sense| {
            Ok(Sense {
                examples: load_examples(connection, &sense.0)?,
                id: sense.0,
                gloss: sense.1,
                definition: sense.2,
                part_of_speech: sense.3,
                semantic_domain: sense.4,
                sort_order: sense.5,
            })
        })
        .collect()
}

fn load_examples(connection: &Connection, sense_id: &str) -> AppResult<Vec<Example>> {
    let mut statement = connection.prepare(
        "SELECT id, translation, notes, sort_order
         FROM examples WHERE sense_id = ?1 ORDER BY sort_order, id",
    )?;
    let rows = statement.query_map(params![sense_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    let raw = rows.collect::<Result<Vec<_>, _>>()?;
    raw.into_iter()
        .map(|example| {
            Ok(Example {
                forms: load_example_forms(connection, &example.0)?,
                id: example.0,
                translation: example.1,
                notes: example.2,
                sort_order: example.3,
            })
        })
        .collect()
}

fn load_example_forms(connection: &Connection, example_id: &str) -> AppResult<Vec<ExampleForm>> {
    let mut statement = connection.prepare(
        "SELECT id, writing_system_id, text, sort_order
         FROM example_forms WHERE example_id = ?1 ORDER BY sort_order, id",
    )?;
    let rows = statement.query_map(params![example_id], |row| {
        Ok(ExampleForm {
            id: row.get(0)?,
            writing_system_id: row.get(1)?,
            text: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_relations(connection: &Connection, entry_id: &str) -> AppResult<Vec<EntryRelation>> {
    let mut statement = connection.prepare(
        "SELECT id, target_entry_id, relation_type, fallback_text, notes, sort_order
         FROM entry_relations WHERE source_entry_id = ?1 ORDER BY sort_order, id",
    )?;
    let rows = statement.query_map(params![entry_id], |row| {
        Ok(EntryRelation {
            id: row.get(0)?,
            target_entry_id: row.get(1)?,
            relation_type: row.get(2)?,
            fallback_text: row.get(3)?,
            notes: row.get(4)?,
            sort_order: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn insert_forms(
    transaction: &Transaction<'_>,
    entry_id: &str,
    forms: &[EntryForm],
) -> AppResult<()> {
    for form in forms {
        let text = normalize_text(&form.text);
        transaction.execute(
            "INSERT INTO entry_forms
             (id, entry_id, writing_system_id, text, search_key, variant_label,
              dialect, status, notes, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                form.id,
                entry_id,
                form.writing_system_id,
                text,
                search_key(&form.text),
                normalize_optional(form.variant_label.clone()),
                normalize_optional(form.dialect.clone()),
                normalize_optional(form.status.clone()),
                normalize_optional(form.notes.clone()),
                form.sort_order
            ],
        )?;
    }
    Ok(())
}

fn insert_senses(transaction: &Transaction<'_>, entry_id: &str, senses: &[Sense]) -> AppResult<()> {
    for sense in senses {
        transaction.execute(
            "INSERT INTO senses
             (id, entry_id, gloss, definition, part_of_speech, semantic_domain, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                sense.id,
                entry_id,
                normalize_optional(sense.gloss.clone()),
                normalize_optional(sense.definition.clone()),
                normalize_optional(sense.part_of_speech.clone()),
                normalize_optional(sense.semantic_domain.clone()),
                sense.sort_order
            ],
        )?;
        for example in &sense.examples {
            transaction.execute(
                "INSERT INTO examples (id, sense_id, translation, notes, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    example.id,
                    sense.id,
                    normalize_optional(example.translation.clone()),
                    normalize_optional(example.notes.clone()),
                    example.sort_order
                ],
            )?;
            for form in &example.forms {
                transaction.execute(
                    "INSERT INTO example_forms
                     (id, example_id, writing_system_id, text, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        form.id,
                        example.id,
                        form.writing_system_id,
                        normalize_text(&form.text),
                        form.sort_order
                    ],
                )?;
            }
        }
    }
    Ok(())
}

fn insert_relations(
    transaction: &Transaction<'_>,
    source_entry_id: &str,
    relations: &[EntryRelation],
) -> AppResult<()> {
    for relation in relations {
        let fallback = if let Some(target_id) = &relation.target_entry_id {
            let target_label: Option<String> = transaction
                .query_row(
                    "SELECT f.text FROM entry_forms f
                     JOIN writing_systems w ON w.id = f.writing_system_id
                     JOIN lexical_entries e ON e.id = f.entry_id
                     WHERE f.entry_id = ?1 AND e.deleted_at IS NULL
                     ORDER BY CASE w.display_role WHEN 'primary' THEN 0 ELSE 1 END,
                              f.sort_order LIMIT 1",
                    params![target_id],
                    |row| row.get(0),
                )
                .optional()?;
            Some(
                relation
                    .fallback_text
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .or(target_label)
                    .ok_or_else(|| {
                        AppError::new("validation", "The relation target is not available.")
                    })?,
            )
        } else {
            normalize_optional(relation.fallback_text.clone())
        };
        transaction.execute(
            "INSERT INTO entry_relations
             (id, source_entry_id, target_entry_id, relation_type, fallback_text, notes, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                relation.id,
                source_entry_id,
                relation.target_entry_id,
                relation.relation_type,
                fallback,
                normalize_optional(relation.notes.clone()),
                relation.sort_order
            ],
        )?;
    }
    Ok(())
}

fn validate_project_name(name: &str) -> AppResult<()> {
    let value = name.trim();
    let windows_stem = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(windows_stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (windows_stem.len() == 4
            && (windows_stem.starts_with("COM") || windows_stem.starts_with("LPT"))
            && matches!(windows_stem.as_bytes()[3], b'1'..=b'9'));
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.ends_with('.')
        || value
            .chars()
            .any(|character| ['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(&character))
        || reserved
    {
        return Err(AppError::new(
            "validation",
            "The project name contains characters that are not portable across Windows and macOS.",
        ));
    }
    Ok(())
}

fn validate_writing_systems(writing_systems: &[WritingSystem]) -> AppResult<()> {
    if writing_systems.is_empty() {
        return Err(AppError::new(
            "validation",
            "At least one writing system is required.",
        ));
    }
    let primary_count = writing_systems
        .iter()
        .filter(|item| item.display_role.as_deref() == Some("primary"))
        .count();
    let secondary_count = writing_systems
        .iter()
        .filter(|item| item.display_role.as_deref() == Some("secondary"))
        .count();
    if primary_count != 1 || secondary_count > 1 {
        return Err(AppError::new(
            "validation",
            "Choose exactly one primary and at most one secondary writing system.",
        ));
    }
    if writing_systems.iter().any(|item| {
        item.name.trim().is_empty()
            || !matches!(
                item.kind.as_str(),
                "orthography"
                    | "romanization"
                    | "transliteration"
                    | "phonemic"
                    | "phonetic"
                    | "other"
            )
    }) {
        return Err(AppError::new(
            "validation",
            "Every writing system needs a name and a supported type.",
        ));
    }
    Ok(())
}

fn validate_metadata_options(options: &[String]) -> AppResult<()> {
    let normalized = options
        .iter()
        .map(|value| normalize_text(value.trim()))
        .collect::<Vec<_>>();
    if normalized.iter().any(String::is_empty) {
        return Err(AppError::new(
            "validation",
            "Metadata options cannot be empty.",
        ));
    }
    let unique = normalized.iter().collect::<std::collections::HashSet<_>>();
    if unique.len() != normalized.len() {
        return Err(AppError::new(
            "validation",
            "Metadata options cannot contain duplicates.",
        ));
    }
    Ok(())
}

fn validate_entry(entry: &LexicalEntry) -> AppResult<()> {
    for relation in &entry.relations {
        if relation.target_entry_id.as_deref() == Some(entry.id.as_str()) {
            return Err(AppError::new(
                "self_relation",
                "An entry cannot have a root or base relation to itself.",
            ));
        }
        if relation.target_entry_id.is_none()
            && relation
                .fallback_text
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(AppError::new(
                "relation_target_required",
                "A relation needs a linked entry or fallback text.",
            ));
        }
        if !matches!(relation.relation_type.as_str(), "root" | "base") {
            return Err(AppError::new(
                "validation",
                "Milestone 1 supports root and base relations only.",
            ));
        }
    }
    Ok(())
}

fn revision_or_not_found(connection: &Connection, id: &str) -> AppResult<AppError> {
    let exists = connection
        .query_row(
            "SELECT 1 FROM lexical_entries WHERE id = ?1",
            params![id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(if exists {
        AppError::new(
            "revision_conflict",
            "This entry changed after it was loaded. Reload it before saving.",
        )
    } else {
        AppError::new("not_found", "The lexical entry was not found.")
    })
}

fn project_id(connection: &Connection) -> AppResult<String> {
    connection
        .query_row("SELECT id FROM projects LIMIT 1", [], |row| row.get(0))
        .map_err(Into::into)
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let normalized = normalize_text(item.trim());
        (!normalized.is_empty()).then_some(normalized)
    })
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::ProjectSession;
    use crate::domain::{
        CreateProjectRequest, DeleteEntryRequest, EntryForm, EntryRelation, Example, ExampleForm,
        SaveEntryRequest, Sense, UpdateProjectSettingsRequest, WritingSystem,
    };

    fn create_session() -> (tempfile::TempDir, ProjectSession) {
        let directory = tempdir().expect("temp directory");
        let session = ProjectSession::create(CreateProjectRequest {
            parent_dir: directory.path().to_string_lossy().into_owned(),
            name: "Test Language".into(),
            language_name: None,
            language_code: None,
        })
        .expect("project creation");
        (directory, session)
    }

    #[test]
    fn saves_and_reopens_unicode_entry_with_nested_examples() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let writing_system_id = snapshot.writing_systems[0].id.clone();
        let pinyin_id = super::new_id();
        let ipa_id = super::new_id();
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name.clone(),
                language_name: Some("Traditional Chinese".into()),
                language_code: Some("zh-Hant".into()),
                description: None,
                writing_systems: vec![
                    snapshot.writing_systems[0].clone(),
                    WritingSystem {
                        id: pinyin_id.clone(),
                        name: "Pinyin".into(),
                        kind: "romanization".into(),
                        script_code: Some("Latn".into()),
                        language_tag: None,
                        display_role: Some("secondary".into()),
                        sort_order: 1,
                        font_family: None,
                        notes: None,
                    },
                    WritingSystem {
                        id: ipa_id.clone(),
                        name: "IPA".into(),
                        kind: "phonetic".into(),
                        script_code: Some("Latn".into()),
                        language_tag: None,
                        display_role: None,
                        sort_order: 2,
                        font_family: None,
                        notes: None,
                    },
                ],
                part_of_speech_options: vec!["Verb".into(), "Noun".into()],
                semantic_domain_options: vec!["Motion".into()],
            })
            .expect("add dynamic writing systems");
        let mut entry = session.create_entry().expect("entry");
        entry.forms.extend([
            EntryForm {
                id: super::new_id(),
                writing_system_id: writing_system_id.clone(),
                text: "過".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 0,
            },
            EntryForm {
                id: super::new_id(),
                writing_system_id: pinyin_id,
                text: "guò".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 1,
            },
            EntryForm {
                id: super::new_id(),
                writing_system_id: ipa_id,
                text: "kuo˥˩".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 2,
            },
        ]);
        entry.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("cross".into()),
            definition: Some("to pass across".into()),
            part_of_speech: Some("Verb".into()),
            semantic_domain: None,
            sort_order: 0,
            examples: vec![Example {
                id: super::new_id(),
                translation: Some("He crossed the river.".into()),
                notes: None,
                sort_order: 0,
                forms: vec![ExampleForm {
                    id: super::new_id(),
                    writing_system_id,
                    text: "他過河了。".into(),
                    sort_order: 0,
                }],
            }],
        });
        let id = entry.id.clone();
        session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .expect("save");
        for query in ["過", "guò", "guo", "kuo˥˩"] {
            assert_eq!(session.query_entries(query).expect("search").len(), 1);
        }

        let root = snapshot.root_path;
        session.close().expect("close");
        let reopened = ProjectSession::open(root).expect("reopen");
        let loaded = reopened.load_entry(&id).expect("load");
        assert_eq!(loaded.forms[0].text, "過");
        assert_eq!(loaded.forms[1].text, "guò");
        assert_eq!(loaded.senses[0].examples[0].forms[0].text, "他過河了。");
        let reopened_snapshot = reopened.snapshot().expect("reopened snapshot");
        assert_eq!(
            reopened_snapshot.part_of_speech_options,
            vec!["Verb", "Noun"]
        );
        assert_eq!(reopened_snapshot.semantic_domain_options, vec!["Motion"]);
    }

    #[test]
    fn stale_revision_does_not_replace_the_aggregate() {
        let (_directory, mut session) = create_session();
        let entry = session.create_entry().expect("entry");
        let saved = session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry: entry.clone(),
            })
            .expect("first save");
        let error = session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .expect_err("stale save must fail");
        assert_eq!(error.code, "revision_conflict");
        assert_eq!(saved.revision, 1);
    }

    #[test]
    fn failed_nested_write_rolls_back_revision_and_children() {
        let (_directory, mut session) = create_session();
        let mut entry = session.create_entry().expect("entry");
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: "missing-writing-system".into(),
            text: "must not persist".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        let id = entry.id.clone();
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect_err("foreign-key violation");
        let loaded = session.load_entry(&id).expect("unchanged entry");
        assert_eq!(loaded.revision, 0);
        assert!(loaded.forms.is_empty());
    }

    #[test]
    fn soft_delete_is_hidden_and_can_be_restored() {
        let (_directory, mut session) = create_session();
        let entry = session.create_entry().expect("entry");
        session
            .delete_entry(DeleteEntryRequest {
                id: entry.id.clone(),
                expected_revision: entry.revision,
            })
            .expect("delete");
        assert!(session.query_entries("").expect("query").is_empty());
        assert_eq!(
            session.load_entry(&entry.id).expect_err("hidden").code,
            "not_found"
        );
        let restored = session.restore_entry(&entry.id).expect("restore");
        assert_eq!(restored.revision, 2);
        assert_eq!(session.query_entries("").expect("query").len(), 1);
    }

    #[test]
    fn backup_is_a_readable_consistent_database() {
        let (_directory, mut session) = create_session();
        session.create_entry().expect("entry");
        let backup = super::create_backup(&session.connection, &session.root).expect("backup");
        let connection = rusqlite::Connection::open(backup).expect("open backup");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM lexical_entries", [], |row| row.get(0))
            .expect("entry count");
        assert_eq!(count, 1);
    }

    #[test]
    fn opening_an_unrelated_sqlite_file_does_not_modify_it() {
        let directory = tempdir().expect("temp directory");
        let root = directory.path().join("NotBkuw.bkuw");
        std::fs::create_dir(&root).expect("project-like folder");
        std::fs::create_dir(root.join("backups")).expect("backups");
        let database_path = root.join("project.sqlite");
        let connection = rusqlite::Connection::open(&database_path).expect("database");
        connection
            .execute("CREATE TABLE unrelated(value TEXT)", [])
            .expect("unrelated schema");
        drop(connection);

        let error = match ProjectSession::open(&root) {
            Err(error) => error,
            Ok(_) => panic!("must reject unrelated database"),
        };
        assert_eq!(error.code, "invalid_project");
        let connection = rusqlite::Connection::open(&database_path).expect("reopen database");
        let migration_table: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'schema_migrations'",
                [],
                |row| row.get(0),
            )
            .expect("schema check");
        assert_eq!(migration_table, 0);
    }

    #[test]
    fn duplicate_project_name_returns_a_stable_error() {
        let directory = tempdir().expect("temp directory");
        let request = CreateProjectRequest {
            parent_dir: directory.path().to_string_lossy().into_owned(),
            name: "Duplicate".into(),
            language_name: None,
            language_code: None,
        };
        let _session = ProjectSession::create(request.clone()).expect("first project");
        let error = match ProjectSession::create(request) {
            Err(error) => error,
            Ok(_) => panic!("duplicate project must not be created"),
        };
        assert_eq!(error.code, "project_exists");
    }

    #[test]
    fn version_one_project_is_backed_up_before_metadata_migration() {
        let (_directory, session) = create_session();
        let root = std::path::PathBuf::from(session.snapshot().expect("snapshot").root_path);
        session.close().expect("close");
        let connection = rusqlite::Connection::open(root.join("project.sqlite")).expect("open");
        connection
            .execute_batch(
                "DROP TABLE metadata_options;
                 DELETE FROM schema_migrations WHERE version = 2;",
            )
            .expect("simulate version one");
        drop(connection);

        let reopened = ProjectSession::open(&root).expect("migrated reopen");
        assert!(
            reopened
                .snapshot()
                .expect("snapshot")
                .part_of_speech_options
                .is_empty()
        );
        let backups = std::fs::read_dir(root.join("backups"))
            .expect("backups")
            .collect::<Result<Vec<_>, _>>()
            .expect("backup entries");
        assert_eq!(backups.len(), 1);
        let backup = rusqlite::Connection::open(backups[0].path()).expect("open backup");
        let version: i64 = backup
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("backup version");
        assert_eq!(version, 1);
    }

    #[test]
    fn a_project_lock_rejects_a_second_writer() {
        let (_directory, session) = create_session();
        let root = session.snapshot().expect("snapshot").root_path;
        let error = match ProjectSession::open(root) {
            Err(error) => error,
            Ok(_) => panic!("a second writer must not open the project"),
        };
        assert_eq!(error.code, "project_locked");
    }

    #[test]
    fn writing_system_roles_require_exactly_one_primary() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let mut unranked = snapshot.writing_systems[0].clone();
        unranked.display_role = None;
        let error = session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name,
                language_name: None,
                language_code: None,
                description: None,
                writing_systems: vec![unranked],
                part_of_speech_options: vec![],
                semantic_domain_options: vec![],
            })
            .expect_err("a primary writing system is required");
        assert_eq!(error.code, "validation");
        assert_eq!(
            session
                .snapshot()
                .expect("unchanged snapshot")
                .writing_systems[0]
                .display_role
                .as_deref(),
            Some("primary")
        );
    }

    #[test]
    fn linked_relation_keeps_a_fallback_label() {
        let (_directory, mut session) = create_session();
        let writing_system_id = session.snapshot().expect("snapshot").writing_systems[0]
            .id
            .clone();
        let mut target = session.create_entry().expect("target");
        target.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id,
            text: "ambuk".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        let target = session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: target,
            })
            .expect("save target");
        let mut source = session.create_entry().expect("source");
        source.relations.push(EntryRelation {
            id: super::new_id(),
            target_entry_id: Some(target.id),
            relation_type: "root".into(),
            fallback_text: None,
            notes: None,
            sort_order: 0,
        });
        let source = session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: source,
            })
            .expect("save source");
        assert_eq!(source.relations[0].fallback_text.as_deref(), Some("ambuk"));
    }

    #[test]
    fn referenced_writing_system_cannot_be_removed() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let original = snapshot.writing_systems[0].clone();
        let replacement = WritingSystem {
            id: super::new_id(),
            name: "IPA".into(),
            kind: "phonetic".into(),
            script_code: Some("Latn".into()),
            language_tag: None,
            display_role: Some("primary".into()),
            sort_order: 1,
            font_family: None,
            notes: None,
        };
        let mut original_unranked = original.clone();
        original_unranked.display_role = None;
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name.clone(),
                language_name: None,
                language_code: None,
                description: None,
                writing_systems: vec![original_unranked, replacement.clone()],
                part_of_speech_options: vec![],
                semantic_domain_options: vec![],
            })
            .expect("add replacement");
        let mut entry = session.create_entry().expect("entry");
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: original.id,
            text: "word".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect("save form");
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name,
                language_name: None,
                language_code: None,
                description: None,
                writing_systems: vec![replacement],
                part_of_speech_options: vec![],
                semantic_domain_options: vec![],
            })
            .expect_err("referenced writing system must be restricted");
        assert_eq!(
            session
                .snapshot()
                .expect("rollback snapshot")
                .writing_systems
                .len(),
            2
        );
    }
}
