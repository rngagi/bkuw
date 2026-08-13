use std::{
    collections::HashMap,
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
        CorpusExportSettings, CreateProjectRequest, DeleteEntryRequest, DeletedEntry, EntryForm,
        EntryRelation, EntrySenseSummary, EntrySortMode, EntrySortSettingsV1, EntrySummary,
        Example, ExampleForm, ExportSettingsV1, FontPreset, LatexExportSettings, LexicalEntry,
        ManualSortLayoutV1, Project, ProjectSnapshot, RelatedEntriesMode, ReverseIndexMode,
        SaveEntryRequest, SectionMode, Sense, UpdateProjectSettingsRequest, WritingSystem,
    },
    error::{AppError, AppResult},
    search::{normalize_text, search_key},
};

const INITIAL_MIGRATION: &str = include_str!("../migrations/001_initial.sql");
const METADATA_OPTIONS_MIGRATION: &str = include_str!("../migrations/002_metadata_options.sql");
const EXPORT_SETTINGS_MIGRATION: &str = include_str!("../migrations/003_export_settings.sql");
const ENTRY_ORDERING_MIGRATION: &str = include_str!("../migrations/004_entry_ordering.sql");
const LATEST_SCHEMA_VERSION: i64 = 4;

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
                 (id, name, language_name, language_code, analysis_language, description, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5, ?5)",
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
        let project = load_project(&self.connection)?;
        let writing_systems = load_writing_systems(&self.connection)?;
        let entry_sort_settings = load_entry_sort_settings(&self.connection, &writing_systems)?;
        let manual_sort_layout = load_manual_sort_layout(&self.connection)?;
        let entries = query_summaries(
            &self.connection,
            "",
            &entry_sort_settings,
            &manual_sort_layout,
            &writing_systems,
        )?;
        Ok(ProjectSnapshot {
            root_path: self.root.to_string_lossy().into_owned(),
            export_settings: load_export_settings(&self.connection, &project, &writing_systems)?,
            entry_sort_settings: entry_sort_settings.clone(),
            manual_sort_layout: manual_sort_layout.clone(),
            project,
            writing_systems,
            part_of_speech_options: load_metadata_options(&self.connection, "part_of_speech")?,
            semantic_domain_options: load_metadata_options(&self.connection, "semantic_domain")?,
            entries,
        })
    }

    pub fn update_settings(
        &mut self,
        request: UpdateProjectSettingsRequest,
    ) -> AppResult<ProjectSnapshot> {
        validate_writing_systems(&request.writing_systems)?;
        validate_metadata_options(&request.part_of_speech_options)?;
        validate_metadata_options(&request.semantic_domain_options)?;
        validate_analysis_language(request.analysis_language.as_deref())?;
        if request.name.trim().is_empty() {
            return Err(AppError::new("validation", "Project name is required."));
        }
        let project_id = project_id(&self.connection)?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "UPDATE projects
             SET name = ?1, language_name = ?2, language_code = ?3,
                 analysis_language = ?4, description = ?5, updated_at = ?6
             WHERE id = ?7",
            params![
                normalize_text(request.name.trim()),
                normalize_optional(request.language_name),
                normalize_optional(request.language_code),
                request.analysis_language,
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

    pub fn save_export_settings(
        &mut self,
        mut settings: ExportSettingsV1,
    ) -> AppResult<ExportSettingsV1> {
        validate_export_settings(&settings, &load_writing_systems(&self.connection)?)?;
        settings.latex.title = normalize_text(settings.latex.title.trim());
        settings.latex.author = normalize_text(settings.latex.author.trim());
        let project_id = project_id(&self.connection)?;
        let encoded = serde_json::to_string(&settings).map_err(|error| {
            AppError::with_details(
                "internal",
                "Export settings could not be encoded.",
                error.to_string(),
            )
        })?;
        self.connection.execute(
            "INSERT INTO export_settings(project_id, version, settings_json, updated_at)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(project_id) DO UPDATE SET
               version = excluded.version,
               settings_json = excluded.settings_json,
               updated_at = excluded.updated_at",
            params![project_id, encoded, now()],
        )?;
        Ok(settings)
    }

    pub fn save_entry_sort_settings(
        &mut self,
        mut settings: EntrySortSettingsV1,
    ) -> AppResult<EntrySortSettingsV1> {
        let systems = load_writing_systems(&self.connection)?;
        let ids = systems.iter().map(|system| system.id.as_str()).collect();
        crate::ordering::validate_settings(&settings, &ids)
            .map_err(|message| AppError::new("validation", message))?;
        settings.alphabet = settings
            .alphabet
            .into_iter()
            .map(|item| normalize_text(item.trim()))
            .filter(|item| !item.is_empty())
            .collect();
        let encoded = serde_json::to_string(&settings).map_err(|error| {
            AppError::with_details(
                "internal",
                "Entry sort settings could not be encoded.",
                error.to_string(),
            )
        })?;
        self.connection.execute(
            "INSERT INTO entry_sort_settings(project_id, version, settings_json, updated_at)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(project_id) DO UPDATE SET version = 1, settings_json = excluded.settings_json, updated_at = excluded.updated_at",
            params![project_id(&self.connection)?, encoded, now()],
        )?;
        Ok(settings)
    }

    pub fn save_manual_sort_layout(
        &mut self,
        layout: ManualSortLayoutV1,
    ) -> AppResult<ManualSortLayoutV1> {
        crate::ordering::validate_layout(&layout)
            .map_err(|message| AppError::new("validation", message))?;
        let encoded = serde_json::to_string(&layout).map_err(|error| {
            AppError::with_details(
                "internal",
                "Manual sort layout could not be encoded.",
                error.to_string(),
            )
        })?;
        self.connection.execute(
            "INSERT INTO manual_sort_layouts(project_id, version, layout_json, updated_at)
             VALUES (?1, 1, ?2, ?3)
             ON CONFLICT(project_id) DO UPDATE SET version = 1, layout_json = excluded.layout_json, updated_at = excluded.updated_at",
            params![project_id(&self.connection)?, encoded, now()],
        )?;
        Ok(layout)
    }

    pub fn query_entries(&self, query: &str) -> AppResult<Vec<EntrySummary>> {
        let systems = load_writing_systems(&self.connection)?;
        query_summaries(
            &self.connection,
            query,
            &load_entry_sort_settings(&self.connection, &systems)?,
            &load_manual_sort_layout(&self.connection)?,
            &systems,
        )
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
             SET notes = ?1, section_override = ?2, revision = revision + 1, updated_at = ?3
             WHERE id = ?4 AND revision = ?5 AND deleted_at IS NULL",
            params![
                normalize_optional(request.entry.notes.clone()),
                normalize_optional(request.entry.section_override.clone()),
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

    #[cfg(test)]
    pub fn preview_export(
        &self,
        kind: crate::domain::ExportKind,
    ) -> AppResult<crate::domain::ExportPreview> {
        crate::export::preview(&self.export_snapshot()?, kind, None)
    }

    #[cfg(test)]
    pub fn preview_export_with_fonts(
        &self,
        kind: crate::domain::ExportKind,
        fonts: &crate::font_manager::FontManager,
    ) -> AppResult<crate::domain::ExportPreview> {
        crate::export::preview(&self.export_snapshot()?, kind, Some(fonts))
    }

    #[cfg(test)]
    pub fn export_project(
        &self,
        request: crate::domain::ExportProjectRequest,
    ) -> AppResult<crate::domain::ExportResult> {
        crate::export::run(&self.export_snapshot()?, request, None)
    }

    #[cfg(test)]
    pub fn export_project_with_fonts(
        &self,
        request: crate::domain::ExportProjectRequest,
        fonts: &crate::font_manager::FontManager,
    ) -> AppResult<crate::domain::ExportResult> {
        crate::export::run(&self.export_snapshot()?, request, Some(fonts))
    }

    pub(crate) fn export_snapshot(&self) -> AppResult<crate::export::ExportSnapshot> {
        let snapshot = self.snapshot()?;
        let sections = snapshot
            .entries
            .iter()
            .map(|entry| (entry.id.clone(), entry.section_label.clone()))
            .collect();
        let mut live_entries = load_live_entries(&self.connection)?;
        let entries = snapshot
            .entries
            .iter()
            .map(|entry| {
                live_entries.remove(&entry.id).ok_or_else(|| {
                    AppError::new(
                        "database",
                        "An entry disappeared while preparing the export.",
                    )
                })
            })
            .collect::<AppResult<Vec<_>>>()?;
        Ok(crate::export::ExportSnapshot {
            project: snapshot.project,
            writing_systems: snapshot.writing_systems,
            settings: snapshot.export_settings,
            sections,
            entries,
        })
    }
}

fn load_live_entries(connection: &Connection) -> AppResult<HashMap<String, LexicalEntry>> {
    let mut forms_by_entry: HashMap<String, Vec<EntryForm>> = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT f.entry_id, f.id, f.writing_system_id, f.text, f.variant_label,
                f.dialect, f.status, f.notes, f.sort_order
         FROM entry_forms f
         JOIN lexical_entries e ON e.id = f.entry_id
         WHERE e.deleted_at IS NULL
         ORDER BY f.entry_id, f.sort_order, f.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            EntryForm {
                id: row.get(1)?,
                writing_system_id: row.get(2)?,
                text: row.get(3)?,
                variant_label: row.get(4)?,
                dialect: row.get(5)?,
                status: row.get(6)?,
                notes: row.get(7)?,
                sort_order: row.get(8)?,
            },
        ))
    })?;
    for row in rows {
        let (entry_id, form) = row?;
        forms_by_entry.entry(entry_id).or_default().push(form);
    }
    drop(statement);

    let mut forms_by_example: HashMap<String, Vec<ExampleForm>> = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT f.example_id, f.id, f.writing_system_id, f.text, f.sort_order
         FROM example_forms f
         JOIN examples x ON x.id = f.example_id
         JOIN senses s ON s.id = x.sense_id
         JOIN lexical_entries e ON e.id = s.entry_id
         WHERE e.deleted_at IS NULL
         ORDER BY f.example_id, f.sort_order, f.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            ExampleForm {
                id: row.get(1)?,
                writing_system_id: row.get(2)?,
                text: row.get(3)?,
                sort_order: row.get(4)?,
            },
        ))
    })?;
    for row in rows {
        let (example_id, form) = row?;
        forms_by_example.entry(example_id).or_default().push(form);
    }
    drop(statement);

    let mut examples_by_sense: HashMap<String, Vec<Example>> = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT x.sense_id, x.id, x.translation, x.notes, x.sort_order
         FROM examples x
         JOIN senses s ON s.id = x.sense_id
         JOIN lexical_entries e ON e.id = s.entry_id
         WHERE e.deleted_at IS NULL
         ORDER BY x.sense_id, x.sort_order, x.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    for row in rows {
        let (sense_id, id, translation, notes, sort_order) = row?;
        let forms = forms_by_example.remove(&id).unwrap_or_default();
        examples_by_sense
            .entry(sense_id)
            .or_default()
            .push(Example {
                id,
                translation,
                notes,
                sort_order,
                forms,
            });
    }
    drop(statement);

    let mut senses_by_entry: HashMap<String, Vec<Sense>> = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT s.entry_id, s.id, s.gloss, s.definition, s.part_of_speech,
                s.semantic_domain, s.sort_order
         FROM senses s
         JOIN lexical_entries e ON e.id = s.entry_id
         WHERE e.deleted_at IS NULL
         ORDER BY s.entry_id, s.sort_order, s.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })?;
    for row in rows {
        let (entry_id, id, gloss, definition, part_of_speech, semantic_domain, sort_order) = row?;
        let examples = examples_by_sense.remove(&id).unwrap_or_default();
        senses_by_entry.entry(entry_id).or_default().push(Sense {
            id,
            gloss,
            definition,
            part_of_speech,
            semantic_domain,
            sort_order,
            examples,
        });
    }
    drop(statement);

    let mut relations_by_entry: HashMap<String, Vec<EntryRelation>> = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT r.source_entry_id, r.id, r.target_entry_id, r.relation_type,
                r.fallback_text, r.notes, r.sort_order
         FROM entry_relations r
         JOIN lexical_entries e ON e.id = r.source_entry_id
         WHERE e.deleted_at IS NULL
         ORDER BY r.source_entry_id, r.sort_order, r.id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            EntryRelation {
                id: row.get(1)?,
                target_entry_id: row.get(2)?,
                relation_type: row.get(3)?,
                fallback_text: row.get(4)?,
                notes: row.get(5)?,
                sort_order: row.get(6)?,
            },
        ))
    })?;
    for row in rows {
        let (entry_id, relation) = row?;
        relations_by_entry
            .entry(entry_id)
            .or_default()
            .push(relation);
    }
    drop(statement);

    let mut entries = HashMap::new();
    let mut statement = connection.prepare(
        "SELECT id, notes, section_override, revision, created_at, updated_at
         FROM lexical_entries WHERE deleted_at IS NULL",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    for row in rows {
        let (id, notes, section_override, revision, created_at, updated_at) = row?;
        entries.insert(
            id.clone(),
            LexicalEntry {
                forms: forms_by_entry.remove(&id).unwrap_or_default(),
                senses: senses_by_entry.remove(&id).unwrap_or_default(),
                relations: relations_by_entry.remove(&id).unwrap_or_default(),
                id,
                notes,
                section_override,
                revision,
                created_at,
                updated_at,
            },
        );
    }
    Ok(entries)
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
    if current < 3 {
        transaction.execute_batch(EXPORT_SETTINGS_MIGRATION)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (3, ?1)",
            params![now()],
        )?;
    }
    if current < 4 {
        transaction.execute_batch(ENTRY_ORDERING_MIGRATION)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES (4, ?1)",
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
            "SELECT id, name, language_name, language_code, analysis_language, description, created_at, updated_at
             FROM projects LIMIT 1",
            [],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    language_name: row.get(2)?,
                    language_code: row.get(3)?,
                    analysis_language: row.get(4)?,
                    description: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .map_err(Into::into)
}

fn default_export_settings(project: &Project, systems: &[WritingSystem]) -> ExportSettingsV1 {
    let primary = systems
        .iter()
        .find(|system| system.display_role.as_deref() == Some("primary"))
        .or_else(|| systems.first());
    let primary_id = primary.map(|system| system.id.clone()).unwrap_or_default();
    let pronunciation = systems
        .iter()
        .find(|system| system.kind == "phonetic")
        .or_else(|| systems.iter().find(|system| system.kind == "phonemic"))
        .map(|system| system.id.clone())
        .filter(|id| id != &primary_id);
    let language_tag = primary.and_then(|system| system.language_tag.clone());
    ExportSettingsV1 {
        version: 1,
        corpus: CorpusExportSettings {
            part_of_speech_mappings: Default::default(),
        },
        latex: LatexExportSettings {
            title: project.name.clone(),
            author: String::new(),
            headword_writing_system_id: primary_id.clone(),
            pronunciation_writing_system_id: pronunciation,
            example_writing_system_id: primary_id,
            collation_language_tag: language_tag,
            section_mode: SectionMode::Auto,
            reverse_index: ReverseIndexMode::Gloss,
            related_entries: RelatedEntriesMode::None,
            font_presets: systems
                .iter()
                .map(|system| (system.id.clone(), FontPreset::Auto))
                .collect(),
        },
    }
}

fn load_export_settings(
    connection: &Connection,
    project: &Project,
    systems: &[WritingSystem],
) -> AppResult<ExportSettingsV1> {
    let encoded = connection
        .query_row(
            "SELECT settings_json FROM export_settings WHERE project_id = ?1",
            params![project.id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match encoded {
        Some(value) => serde_json::from_str(&value)
            .map(|settings| normalize_export_settings(settings, project, systems))
            .map_err(|error| {
                AppError::with_details(
                    "database",
                    "Export settings are invalid.",
                    error.to_string(),
                )
            }),
        None => Ok(default_export_settings(project, systems)),
    }
}

fn normalize_export_settings(
    mut settings: ExportSettingsV1,
    project: &Project,
    systems: &[WritingSystem],
) -> ExportSettingsV1 {
    let defaults = default_export_settings(project, systems);
    let ids = systems
        .iter()
        .map(|system| system.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if !ids.contains(settings.latex.headword_writing_system_id.as_str()) {
        settings.latex.headword_writing_system_id =
            defaults.latex.headword_writing_system_id.clone();
    }
    if !ids.contains(settings.latex.example_writing_system_id.as_str()) {
        settings.latex.example_writing_system_id = defaults.latex.example_writing_system_id;
    }
    if settings
        .latex
        .pronunciation_writing_system_id
        .as_deref()
        .is_some_and(|id| !ids.contains(id))
    {
        settings.latex.pronunciation_writing_system_id =
            defaults.latex.pronunciation_writing_system_id.clone();
    }
    if settings.latex.pronunciation_writing_system_id.as_deref()
        == Some(settings.latex.headword_writing_system_id.as_str())
    {
        settings.latex.pronunciation_writing_system_id = defaults
            .latex
            .pronunciation_writing_system_id
            .filter(|id| id != &settings.latex.headword_writing_system_id);
    }
    settings
        .latex
        .font_presets
        .retain(|id, _| ids.contains(id.as_str()));
    for system in systems {
        settings
            .latex
            .font_presets
            .entry(system.id.clone())
            .or_insert(FontPreset::Auto);
    }
    settings
}

fn validate_analysis_language(value: Option<&str>) -> AppResult<()> {
    if value.is_some_and(|language| !matches!(language, "zh-TW" | "en")) {
        return Err(AppError::new(
            "validation",
            "Analysis language must be zh-TW or en.",
        ));
    }
    Ok(())
}

fn validate_export_settings(
    settings: &ExportSettingsV1,
    systems: &[WritingSystem],
) -> AppResult<()> {
    if settings.version != 1 {
        return Err(AppError::new(
            "validation",
            "Unsupported export settings version.",
        ));
    }
    let ids = systems
        .iter()
        .map(|system| system.id.as_str())
        .collect::<Vec<_>>();
    for id in [
        Some(settings.latex.headword_writing_system_id.as_str()),
        settings.latex.pronunciation_writing_system_id.as_deref(),
        Some(settings.latex.example_writing_system_id.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        if !ids.contains(&id) {
            return Err(AppError::new(
                "validation",
                "Export settings reference a missing writing system.",
            ));
        }
    }
    if settings.latex.pronunciation_writing_system_id.as_deref()
        == Some(settings.latex.headword_writing_system_id.as_str())
    {
        return Err(AppError::new(
            "validation",
            "Headword and pronunciation must use different writing systems.",
        ));
    }
    Ok(())
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

fn default_entry_sort_settings(systems: &[WritingSystem]) -> EntrySortSettingsV1 {
    let writing_system_id = systems
        .iter()
        .find(|system| system.display_role.as_deref() == Some("primary"))
        .or_else(|| systems.first())
        .map(|system| system.id.clone())
        .unwrap_or_default();
    EntrySortSettingsV1 {
        version: 1,
        mode: EntrySortMode::Auto,
        writing_system_id,
        alphabet: Vec::new(),
    }
}

fn load_entry_sort_settings(
    connection: &Connection,
    systems: &[WritingSystem],
) -> AppResult<EntrySortSettingsV1> {
    let encoded = connection
        .query_row(
            "SELECT settings_json FROM entry_sort_settings WHERE project_id = ?1",
            params![project_id(connection)?],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match encoded {
        None => Ok(default_entry_sort_settings(systems)),
        Some(value) => {
            let settings: EntrySortSettingsV1 = serde_json::from_str(&value).map_err(|error| {
                AppError::with_details(
                    "database",
                    "Entry sort settings are invalid.",
                    error.to_string(),
                )
            })?;
            let ids = systems.iter().map(|system| system.id.as_str()).collect();
            if crate::ordering::validate_settings(&settings, &ids).is_ok() {
                Ok(settings)
            } else {
                Ok(default_entry_sort_settings(systems))
            }
        }
    }
}

fn load_manual_sort_layout(connection: &Connection) -> AppResult<ManualSortLayoutV1> {
    let encoded = connection
        .query_row(
            "SELECT layout_json FROM manual_sort_layouts WHERE project_id = ?1",
            params![project_id(connection)?],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    match encoded {
        None => Ok(ManualSortLayoutV1 {
            version: 1,
            items: Vec::new(),
        }),
        Some(value) => serde_json::from_str(&value).map_err(|error| {
            AppError::with_details(
                "database",
                "Manual sort layout is invalid.",
                error.to_string(),
            )
        }),
    }
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

fn query_summaries(
    connection: &Connection,
    query: &str,
    settings: &EntrySortSettingsV1,
    layout: &ManualSortLayoutV1,
    systems: &[WritingSystem],
) -> AppResult<Vec<EntrySummary>> {
    let key = search_key(query.trim());
    let pattern = format!("%{key}%");
    let mut senses_by_entry = load_entry_sense_summaries(connection, &key, &pattern)?;
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
                (SELECT f.writing_system_id FROM entry_forms f
                 JOIN writing_systems w ON w.id = f.writing_system_id
                 WHERE f.entry_id = e.id AND w.type IN ('phonetic', 'phonemic')
                 ORDER BY CASE w.type WHEN 'phonetic' THEN 0 ELSE 1 END,
                          w.sort_order, f.sort_order LIMIT 1),
                (SELECT f.text FROM entry_forms f
                 JOIN writing_systems w ON w.id = f.writing_system_id
                 WHERE f.entry_id = e.id AND w.type IN ('phonetic', 'phonemic')
                 ORDER BY CASE w.type WHEN 'phonetic' THEN 0 ELSE 1 END,
                          w.sort_order, f.sort_order LIMIT 1),
                e.revision
                ,e.section_override
                ,COALESCE((SELECT f.text FROM entry_forms f
                           WHERE f.entry_id = e.id AND f.writing_system_id = ?3
                           ORDER BY f.sort_order LIMIT 1), '')
         FROM lexical_entries e
         WHERE e.deleted_at IS NULL
           AND (?1 = '' OR EXISTS (
             SELECT 1 FROM entry_forms f
             WHERE f.entry_id = e.id AND f.search_key LIKE ?2
           ))
         ORDER BY e.created_at, e.id",
    )?;
    let rows = statement.query_map(params![key, pattern, settings.writing_system_id], |row| {
        let id: String = row.get(0)?;
        Ok(crate::ordering::SortableSummary {
            summary: EntrySummary {
                senses: senses_by_entry.remove(&id).unwrap_or_default(),
                id,
                primary_form: row.get(1)?,
                secondary_form: row.get(2)?,
                pronunciation_writing_system_id: row.get(3)?,
                pronunciation_form: row.get(4)?,
                revision: row.get(5)?,
                section_label: None,
                manual_order_pending: false,
            },
            section_override: row.get(6)?,
            sort_text: row.get::<_, String>(7)?,
        })
    })?;
    let rows = rows.collect::<Result<Vec<_>, _>>()?;
    let language_tag = systems
        .iter()
        .find(|system| system.id == settings.writing_system_id)
        .and_then(|system| system.language_tag.as_deref());
    Ok(crate::ordering::order_summaries(
        rows,
        settings,
        layout,
        language_tag,
    ))
}

fn load_entry_sense_summaries(
    connection: &Connection,
    search_key: &str,
    search_pattern: &str,
) -> AppResult<HashMap<String, Vec<EntrySenseSummary>>> {
    let mut statement = connection.prepare(
        "SELECT s.entry_id, s.part_of_speech, s.gloss
         FROM senses s
         JOIN lexical_entries e ON e.id = s.entry_id
         WHERE e.deleted_at IS NULL
           AND (?1 = '' OR EXISTS (
             SELECT 1 FROM entry_forms f
             WHERE f.entry_id = e.id AND f.search_key LIKE ?2
           ))
         ORDER BY s.entry_id, s.sort_order, s.id",
    )?;
    let rows = statement.query_map(params![search_key, search_pattern], |row| {
        Ok((
            row.get::<_, String>(0)?,
            EntrySenseSummary {
                part_of_speech: row.get(1)?,
                gloss: row.get(2)?,
            },
        ))
    })?;
    let mut summaries: HashMap<String, Vec<EntrySenseSummary>> = HashMap::new();
    for row in rows {
        let (entry_id, summary) = row?;
        summaries.entry(entry_id).or_default().push(summary);
    }
    Ok(summaries)
}

fn load_entry(connection: &Connection, id: &str, include_deleted: bool) -> AppResult<LexicalEntry> {
    let entry = connection
        .query_row(
            "SELECT id, notes, section_override, revision, created_at, updated_at
             FROM lexical_entries
             WHERE id = ?1 AND (?2 OR deleted_at IS NULL)",
            params![id, include_deleted],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::new("not_found", "The lexical entry was not found."))?;

    Ok(LexicalEntry {
        id: entry.0,
        notes: entry.1,
        section_override: entry.2,
        revision: entry.3,
        created_at: entry.4,
        updated_at: entry.5,
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
            || item.script_code.as_deref().is_some_and(|code| {
                let bytes = code.as_bytes();
                bytes.len() != 4
                    || !bytes[0].is_ascii_uppercase()
                    || !bytes[1..].iter().all(u8::is_ascii_lowercase)
            })
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
        CorpusPartOfSpeech, CreateProjectRequest, DeleteEntryRequest, EntryForm, EntryRelation,
        EntrySortMode, EntrySortSettingsV1, Example, ExampleForm, ExportKind, ExportProjectRequest,
        FontPreset, ManualSortItem, ManualSortLayoutV1, RelatedEntriesMode, SaveEntryRequest,
        Sense, UpdateProjectSettingsRequest, WritingSystem,
    };
    use crate::font_manager::FontManager;

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
    fn manual_sort_items_use_the_frontend_camel_case_contract() {
        let item = ManualSortItem::Entry {
            entry_id: "entry-1".into(),
        };
        let encoded = serde_json::to_value(&item).expect("serialize item");
        assert_eq!(encoded["kind"], "entry");
        assert_eq!(encoded["entryId"], "entry-1");
        assert!(encoded.get("entry_id").is_none());
        assert_eq!(
            serde_json::from_value::<ManualSortItem>(serde_json::json!({
                "kind": "entry",
                "entryId": "entry-1",
            }))
            .expect("frontend payload"),
            item,
        );
    }

    #[test]
    fn custom_alphabet_sorts_multigraphs_and_supplies_section_labels() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        session
            .save_entry_sort_settings(EntrySortSettingsV1 {
                version: 1,
                mode: EntrySortMode::Auto,
                writing_system_id: primary_id.clone(),
                alphabet: vec!["a".into(), "b".into(), "c".into(), "n".into(), "ng".into()],
            })
            .expect("sort settings");

        for text in ["ngungu", "naga", "caxa", "baba", "ama", "ata", "ngayon"] {
            let mut entry = session.create_entry().expect("entry");
            entry.forms.push(EntryForm {
                id: super::new_id(),
                writing_system_id: primary_id.clone(),
                text: text.into(),
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
                .expect("save entry");
        }

        let summaries = session.query_entries("").expect("summaries");
        assert_eq!(
            summaries
                .iter()
                .map(|item| item.primary_form.as_str())
                .collect::<Vec<_>>(),
            vec!["ama", "ata", "baba", "caxa", "naga", "ngayon", "ngungu"]
        );
        assert_eq!(
            summaries
                .iter()
                .map(|item| item.section_label.as_deref())
                .collect::<Vec<_>>(),
            vec![
                Some("A"),
                Some("A"),
                Some("B"),
                Some("C"),
                Some("N"),
                Some("NG"),
                Some("NG")
            ]
        );
    }

    #[test]
    fn section_override_regroups_without_changing_natural_order() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        session
            .save_entry_sort_settings(EntrySortSettingsV1 {
                version: 1,
                mode: EntrySortMode::Auto,
                writing_system_id: primary_id.clone(),
                alphabet: vec!["n".into(), "ng".into()],
            })
            .expect("settings");
        for (text, override_label) in [("naga", None), ("ngayon", None), ("ngungu", Some("N"))] {
            let mut entry = session.create_entry().expect("entry");
            entry.section_override = override_label.map(str::to_owned);
            entry.forms.push(EntryForm {
                id: super::new_id(),
                writing_system_id: primary_id.clone(),
                text: text.into(),
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
                .expect("save");
        }
        let summaries = session.query_entries("").expect("summaries");
        assert_eq!(
            summaries
                .iter()
                .map(|item| (item.primary_form.as_str(), item.section_label.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("naga", Some("N")),
                ("ngungu", Some("N")),
                ("ngayon", Some("NG")),
            ]
        );
    }

    #[test]
    fn manual_layout_orders_headings_and_marks_new_entries_pending() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        let mut ids = Vec::new();
        for text in ["ama", "baba"] {
            let mut entry = session.create_entry().expect("entry");
            ids.push(entry.id.clone());
            entry.forms.push(EntryForm {
                id: super::new_id(),
                writing_system_id: primary_id.clone(),
                text: text.into(),
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
                .expect("save");
        }
        session
            .save_manual_sort_layout(ManualSortLayoutV1 {
                version: 1,
                items: vec![
                    ManualSortItem::Heading {
                        id: "special".into(),
                        label: "Special".into(),
                    },
                    ManualSortItem::Entry {
                        entry_id: ids[1].clone(),
                    },
                    ManualSortItem::Heading {
                        id: "regular".into(),
                        label: "Regular".into(),
                    },
                    ManualSortItem::Entry {
                        entry_id: ids[0].clone(),
                    },
                ],
            })
            .expect("layout");
        session
            .save_entry_sort_settings(EntrySortSettingsV1 {
                version: 1,
                mode: EntrySortMode::Manual,
                writing_system_id: primary_id.clone(),
                alphabet: vec!["a".into(), "b".into(), "c".into()],
            })
            .expect("settings");

        let mut entry = session.create_entry().expect("new entry");
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: primary_id,
            text: "caxa".into(),
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
            .expect("save new");

        let summaries = session.query_entries("").expect("summaries");
        assert_eq!(
            summaries
                .iter()
                .map(|item| item.primary_form.as_str())
                .collect::<Vec<_>>(),
            vec!["baba", "ama", "caxa"]
        );
        assert_eq!(
            summaries
                .iter()
                .map(|item| item.section_label.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("Special"), Some("Regular"), Some("C")]
        );
        assert_eq!(
            summaries
                .iter()
                .map(|item| item.manual_order_pending)
                .collect::<Vec<_>>(),
            vec![false, false, true]
        );
    }

    #[test]
    fn migration_four_sort_settings_layout_and_override_survive_reopen() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        let mut entry = session.create_entry().expect("entry");
        let entry_id = entry.id.clone();
        entry.section_override = Some("N".into());
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: primary_id.clone(),
            text: "ngungu".into(),
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
            .expect("entry save");
        let settings = EntrySortSettingsV1 {
            version: 1,
            mode: EntrySortMode::Manual,
            writing_system_id: primary_id,
            alphabet: vec!["n".into(), "ng".into()],
        };
        let layout = ManualSortLayoutV1 {
            version: 1,
            items: vec![
                ManualSortItem::Heading {
                    id: "n".into(),
                    label: "N".into(),
                },
                ManualSortItem::Entry {
                    entry_id: entry_id.clone(),
                },
            ],
        };
        session
            .save_entry_sort_settings(settings.clone())
            .expect("settings");
        session
            .save_manual_sort_layout(layout.clone())
            .expect("layout");
        let root = session.snapshot().expect("snapshot").root_path;
        session.close().expect("close");

        let reopened = ProjectSession::open(root).expect("reopen");
        let snapshot = reopened.snapshot().expect("snapshot");
        assert_eq!(snapshot.entry_sort_settings, settings);
        assert_eq!(snapshot.manual_sort_layout, layout);
        assert_eq!(
            reopened
                .load_entry(&entry_id)
                .expect("entry")
                .section_override
                .as_deref(),
            Some("N")
        );
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
                analysis_language: None,
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
        let export_snapshot = reopened.export_snapshot().expect("bulk export snapshot");
        assert_eq!(export_snapshot.entries, vec![loaded]);
        let reopened_snapshot = reopened.snapshot().expect("reopened snapshot");
        assert_eq!(
            reopened_snapshot.part_of_speech_options,
            vec!["Verb", "Noun"]
        );
        assert_eq!(reopened_snapshot.semantic_domain_options, vec!["Motion"]);
    }

    #[test]
    fn entry_summaries_keep_pronunciation_and_ordered_sense_metadata() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        let ipa_id = super::new_id();
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name,
                language_name: Some("Test Language".into()),
                language_code: Some("und".into()),
                analysis_language: Some("zh-TW".into()),
                description: None,
                writing_systems: vec![
                    snapshot.writing_systems[0].clone(),
                    WritingSystem {
                        id: ipa_id.clone(),
                        name: "IPA".into(),
                        kind: "phonemic".into(),
                        script_code: Some("Latn".into()),
                        language_tag: None,
                        display_role: Some("secondary".into()),
                        sort_order: 1,
                        font_family: None,
                        notes: None,
                    },
                ],
                part_of_speech_options: vec!["Noun".into(), "Verb".into()],
                semantic_domain_options: Vec::new(),
            })
            .expect("settings");
        let mut entry = session.create_entry().expect("entry");
        entry.forms.extend([
            EntryForm {
                id: super::new_id(),
                writing_system_id: primary_id,
                text: "ata".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 0,
            },
            EntryForm {
                id: super::new_id(),
                writing_system_id: ipa_id.clone(),
                text: "ata".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 1,
            },
        ]);
        entry.senses.extend([
            Sense {
                id: super::new_id(),
                gloss: Some("父親".into()),
                definition: None,
                part_of_speech: Some("Noun".into()),
                semantic_domain: None,
                sort_order: 0,
                examples: Vec::new(),
            },
            Sense {
                id: super::new_id(),
                gloss: Some("稱作父親".into()),
                definition: None,
                part_of_speech: Some("Verb".into()),
                semantic_domain: None,
                sort_order: 1,
                examples: Vec::new(),
            },
        ]);
        session
            .save_entry(SaveEntryRequest {
                expected_revision: entry.revision,
                entry,
            })
            .expect("save");

        let summaries = session.query_entries("").expect("summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].pronunciation_form.as_deref(), Some("ata"));
        assert_eq!(
            summaries[0].pronunciation_writing_system_id.as_deref(),
            Some(ipa_id.as_str())
        );
        assert_eq!(
            summaries[0]
                .senses
                .iter()
                .map(|sense| (sense.part_of_speech.as_deref(), sense.gloss.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                (Some("Noun"), Some("父親")),
                (Some("Verb"), Some("稱作父親"))
            ]
        );
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
                "DROP TABLE manual_sort_layouts;
                 DROP TABLE entry_sort_settings;
                 ALTER TABLE lexical_entries DROP COLUMN section_override;
                 DROP TABLE export_settings;
                 ALTER TABLE projects DROP COLUMN analysis_language;
                 DROP TABLE metadata_options;
                 DELETE FROM schema_migrations WHERE version >= 2;",
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
    fn migration_three_persists_analysis_language_and_export_settings() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        assert_eq!(snapshot.project.analysis_language, None);
        assert_eq!(snapshot.export_settings.version, 1);

        let mut settings = snapshot.export_settings.clone();
        settings.latex.title = "Field dictionary".into();
        session
            .save_export_settings(settings.clone())
            .expect("save export settings");
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name,
                language_name: snapshot.project.language_name,
                language_code: snapshot.project.language_code,
                analysis_language: Some("zh-TW".into()),
                description: snapshot.project.description,
                writing_systems: snapshot.writing_systems,
                part_of_speech_options: snapshot.part_of_speech_options,
                semantic_domain_options: snapshot.semantic_domain_options,
            })
            .expect("save analysis language");

        let persisted = session.snapshot().expect("persisted snapshot");
        assert_eq!(
            persisted.project.analysis_language.as_deref(),
            Some("zh-TW")
        );
        assert_eq!(persisted.export_settings, settings);
        let root = persisted.root_path;
        session.close().expect("close");
        let reopened = ProjectSession::open(root).expect("reopen");
        let reopened_snapshot = reopened.snapshot().expect("reopened snapshot");
        assert_eq!(
            reopened_snapshot.project.analysis_language.as_deref(),
            Some("zh-TW")
        );
        assert_eq!(reopened_snapshot.export_settings, settings);
    }

    #[test]
    fn older_export_profile_defaults_related_entries_to_none() {
        let (_directory, session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let mut value = serde_json::to_value(&snapshot.export_settings).expect("json");
        value
            .get_mut("latex")
            .and_then(serde_json::Value::as_object_mut)
            .expect("latex object")
            .remove("relatedEntries");
        session
            .connection
            .execute(
                "INSERT INTO export_settings(project_id, version, settings_json, updated_at)
             VALUES (?1, 1, ?2, ?3)",
                rusqlite::params![snapshot.project.id, value.to_string(), super::now()],
            )
            .expect("legacy profile");
        assert_eq!(
            session
                .snapshot()
                .expect("loaded profile")
                .export_settings
                .latex
                .related_entries,
            RelatedEntriesMode::None,
        );
    }

    #[test]
    fn export_profile_keeps_headword_and_pronunciation_systems_distinct() {
        let (_directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let mut settings = snapshot.export_settings.clone();
        settings.latex.pronunciation_writing_system_id =
            Some(settings.latex.headword_writing_system_id.clone());
        let error = session
            .save_export_settings(settings.clone())
            .expect_err("duplicate writing-system selection");
        assert_eq!(error.code, "validation");

        session
            .connection
            .execute(
                "INSERT INTO export_settings(project_id, version, settings_json, updated_at)
                 VALUES (?1, 1, ?2, ?3)",
                rusqlite::params![
                    snapshot.project.id,
                    serde_json::to_string(&settings).expect("settings json"),
                    super::now(),
                ],
            )
            .expect("legacy duplicate profile");
        assert_eq!(
            session
                .snapshot()
                .expect("normalized profile")
                .export_settings
                .latex
                .pronunciation_writing_system_id,
            None,
        );
    }

    #[test]
    fn exports_exact_rngagi_corpus_v03_csv_from_senses() {
        let (directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        let ipa_id = super::new_id();
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name,
                language_name: Some("Test Language".into()),
                language_code: Some("und".into()),
                analysis_language: Some("zh-TW".into()),
                description: None,
                writing_systems: vec![
                    WritingSystem {
                        script_code: Some("Hant".into()),
                        language_tag: Some("zh-Hant".into()),
                        ..snapshot.writing_systems[0].clone()
                    },
                    WritingSystem {
                        id: ipa_id.clone(),
                        name: "IPA".into(),
                        kind: "phonetic".into(),
                        script_code: Some("Latn".into()),
                        language_tag: None,
                        display_role: None,
                        sort_order: 1,
                        font_family: None,
                        notes: None,
                    },
                ],
                part_of_speech_options: vec!["動詞".into()],
                semantic_domain_options: vec!["移動".into()],
            })
            .expect("settings");
        let mut export_settings = session.snapshot().expect("snapshot").export_settings;
        export_settings.latex.pronunciation_writing_system_id = Some(ipa_id.clone());
        export_settings
            .corpus
            .part_of_speech_mappings
            .insert("動詞".into(), CorpusPartOfSpeech::Verb);
        session
            .save_export_settings(export_settings)
            .expect("export settings");

        let mut entry = session.create_entry().expect("entry");
        entry.notes = Some("field note".into());
        entry.forms = vec![
            EntryForm {
                id: super::new_id(),
                writing_system_id: primary_id.clone(),
                text: "過".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 0,
            },
            EntryForm {
                id: super::new_id(),
                writing_system_id: ipa_id,
                text: "kuo˥˩".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 1,
            },
        ];
        entry.relations.push(EntryRelation {
            id: super::new_id(),
            target_entry_id: None,
            relation_type: "root".into(),
            fallback_text: Some("guo".into()),
            notes: None,
            sort_order: 0,
        });
        entry.senses = vec![
            Sense {
                id: super::new_id(),
                gloss: Some("通過".into()),
                definition: Some("從一側到另一側".into()),
                part_of_speech: Some("動詞".into()),
                semantic_domain: Some("移動".into()),
                sort_order: 0,
                examples: vec![Example {
                    id: super::new_id(),
                    translation: Some("他過河了。".into()),
                    notes: Some("elicited".into()),
                    sort_order: 0,
                    forms: vec![ExampleForm {
                        id: super::new_id(),
                        writing_system_id: primary_id.clone(),
                        text: "他過河了。".into(),
                        sort_order: 0,
                    }],
                }],
            },
            Sense {
                id: super::new_id(),
                gloss: Some("經歷".into()),
                definition: None,
                part_of_speech: Some("動詞".into()),
                semantic_domain: None,
                sort_order: 1,
                examples: vec![],
            },
        ];
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect("save");

        let preview = session
            .preview_export(ExportKind::CorpusCsv)
            .expect("preview");
        assert_eq!(preview.row_count, 2);
        assert!(!preview.has_errors(), "{:#?}", preview.issues);
        let output = directory.path().join("corpus.csv");
        let result = session
            .export_project(ExportProjectRequest {
                kind: ExportKind::CorpusCsv,
                destination: output.to_string_lossy().into_owned(),
                snapshot_token: preview.snapshot_token,
                overwrite: false,
            })
            .expect("export");
        assert_eq!(result.row_count, 2);
        assert_eq!(
            std::fs::read_to_string(output).expect("CSV"),
            concat!(
                "form,gloss_zh,word_root,example,example_translation_zh,ipa,part_of_speech,gloss_en,notes\r\n",
                "過,通過,guo,他過河了。,他過河了。,kuo˥˩,verb,,entry_notes: field note\\nsense_definition: 從一側到另一側\\nsemantic_domain: 移動\\nexample_notes: elicited\r\n",
                "過,經歷,guo,,,kuo˥˩,verb,,entry_notes: field note\r\n",
            )
        );
    }

    #[test]
    fn corpus_preview_reports_loss_and_rejects_stale_or_invalid_exports() {
        let (directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        let mut entry = session.create_entry().expect("entry");
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: primary_id.clone(),
            text: "a,\"b\nc".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        entry.relations.push(EntryRelation {
            id: super::new_id(),
            target_entry_id: None,
            relation_type: "base".into(),
            fallback_text: Some("base".into()),
            notes: None,
            sort_order: 0,
        });
        entry.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("有,引號\"".into()),
            definition: None,
            part_of_speech: Some("動詞".into()),
            semantic_domain: None,
            sort_order: 0,
            examples: vec![Example {
                id: super::new_id(),
                translation: None,
                notes: None,
                sort_order: 0,
                forms: vec![ExampleForm {
                    id: super::new_id(),
                    writing_system_id: primary_id,
                    text: "不完整".into(),
                    sort_order: 0,
                }],
            }],
        });
        let saved = session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect("save");

        let invalid = session
            .preview_export(ExportKind::CorpusCsv)
            .expect("preview");
        assert!(
            invalid
                .issues
                .iter()
                .any(|item| item.code == "corpus.analysis_language_required")
        );
        let error = session
            .export_project(ExportProjectRequest {
                kind: ExportKind::CorpusCsv,
                destination: directory
                    .path()
                    .join("invalid.csv")
                    .to_string_lossy()
                    .into_owned(),
                snapshot_token: invalid.snapshot_token,
                overwrite: false,
            })
            .expect_err("invalid export");
        assert_eq!(error.code, "export_validation");

        let current = session.snapshot().expect("snapshot");
        let current_primary_id = current.writing_systems[0].id.clone();
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: current.project.name,
                language_name: current.project.language_name,
                language_code: current.project.language_code,
                analysis_language: Some("zh-TW".into()),
                description: current.project.description,
                writing_systems: current.writing_systems,
                part_of_speech_options: vec!["動詞".into()],
                semantic_domain_options: current.semantic_domain_options,
            })
            .expect("analysis language");
        let mut deleted = session.create_entry().expect("deleted entry");
        deleted.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: current_primary_id,
            text: "deleted".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        deleted.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("已刪除".into()),
            definition: None,
            part_of_speech: None,
            semantic_domain: None,
            sort_order: 0,
            examples: vec![],
        });
        let deleted = session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: deleted,
            })
            .expect("save deleted candidate");
        session
            .delete_entry(DeleteEntryRequest {
                id: deleted.id,
                expected_revision: deleted.revision,
            })
            .expect("soft delete");
        let preview = session
            .preview_export(ExportKind::CorpusCsv)
            .expect("valid preview");
        assert_eq!(
            preview.row_count, 1,
            "soft-deleted entries must be excluded"
        );
        assert!(!preview.has_errors());
        assert!(
            preview
                .issues
                .iter()
                .any(|item| item.code == "corpus.part_of_speech_unmapped")
        );
        assert!(
            preview
                .issues
                .iter()
                .any(|item| item.code == "corpus.examples_omitted")
        );
        assert!(
            preview
                .issues
                .iter()
                .any(|item| item.code == "corpus.base_relations_omitted")
        );

        let mut changed = session.load_entry(&saved.id).expect("entry");
        changed.notes = Some("changed after preview".into());
        session
            .save_entry(SaveEntryRequest {
                expected_revision: changed.revision,
                entry: changed,
            })
            .expect("change");
        let output = directory.path().join("quoted.csv");
        let stale = session
            .export_project(ExportProjectRequest {
                kind: ExportKind::CorpusCsv,
                destination: output.to_string_lossy().into_owned(),
                snapshot_token: preview.snapshot_token,
                overwrite: false,
            })
            .expect_err("stale preview");
        assert_eq!(stale.code, "export_stale");

        let fresh = session
            .preview_export(ExportKind::CorpusCsv)
            .expect("fresh preview");
        session
            .export_project(ExportProjectRequest {
                kind: ExportKind::CorpusCsv,
                destination: output.to_string_lossy().into_owned(),
                snapshot_token: fresh.snapshot_token.clone(),
                overwrite: false,
            })
            .expect("export");
        let exists = session
            .export_project(ExportProjectRequest {
                kind: ExportKind::CorpusCsv,
                destination: output.to_string_lossy().into_owned(),
                snapshot_token: fresh.snapshot_token.clone(),
                overwrite: false,
            })
            .expect_err("existing destination needs confirmation");
        assert_eq!(exists.code, "export_filesystem");
        assert_eq!(exists.details.as_deref(), Some("destination_exists"));
        session
            .export_project(ExportProjectRequest {
                kind: ExportKind::CorpusCsv,
                destination: output.to_string_lossy().into_owned(),
                snapshot_token: fresh.snapshot_token,
                overwrite: true,
            })
            .expect("confirmed atomic replacement");
        let bytes = std::fs::read(output).expect("CSV");
        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert!(bytes.windows(2).any(|window| window == b"\r\n"));
        let mut reader = csv::Reader::from_reader(bytes.as_slice());
        let record = reader.records().next().expect("row").expect("valid CSV");
        assert_eq!(&record[0], "a,\"b\nc");
        assert_eq!(&record[1], "有,引號\"");
    }

    #[test]
    fn exports_portable_xelatex_project_and_overleaf_zip() {
        let (directory, mut session) = create_session();
        let mut snapshot = session.snapshot().expect("snapshot");
        snapshot.writing_systems[0].script_code = Some("Hant".into());
        let ipa_id = super::new_id();
        let mut writing_systems = snapshot.writing_systems.clone();
        writing_systems.push(WritingSystem {
            id: ipa_id.clone(),
            name: "IPA".into(),
            kind: "phonetic".into(),
            script_code: Some("Latn".into()),
            language_tag: None,
            display_role: None,
            sort_order: 1,
            font_family: None,
            notes: None,
        });
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name.clone(),
                language_name: snapshot.project.language_name.clone(),
                language_code: snapshot.project.language_code.clone(),
                analysis_language: Some("zh-TW".into()),
                description: snapshot.project.description.clone(),
                writing_systems,
                part_of_speech_options: vec!["verb".into()],
                semantic_domain_options: vec![],
            })
            .expect("Hant settings");
        let writing_system_id = snapshot.writing_systems[0].id.clone();
        let mut settings = session
            .snapshot()
            .expect("updated snapshot")
            .export_settings;
        settings.latex.title = "Field #1 & notes".into();
        settings.latex.author = "A_B".into();
        settings.latex.pronunciation_writing_system_id = Some(ipa_id.clone());
        settings
            .latex
            .font_presets
            .insert(writing_system_id.clone(), FontPreset::ChironSungHk);
        session.save_export_settings(settings).expect("settings");
        let mut entry = session.create_entry().expect("entry");
        entry.notes = Some("詞條備註".into());
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: writing_system_id.clone(),
            text: "過_#%".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: ipa_id,
            text: "kuo˥˩".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 1,
        });
        entry.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("通過 & 經歷".into()),
            definition: Some("\\test {value}".into()),
            part_of_speech: Some("verb".into()),
            semantic_domain: None,
            sort_order: 0,
            examples: vec![Example {
                id: super::new_id(),
                translation: Some("他過河了。".into()),
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
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect("save");

        let fonts = FontManager::seeded_for_tests(
            directory.path().join("font-cache"),
            &[
                "tex-gyre-termes",
                "noto-serif-cjk-tc",
                "charis-sil",
                "chiron-sung-hk",
            ],
        );
        let preview = session
            .preview_export_with_fonts(ExportKind::Latex, &fonts)
            .expect("preview");
        let result = session
            .export_project_with_fonts(
                ExportProjectRequest {
                    kind: ExportKind::Latex,
                    destination: directory.path().to_string_lossy().into_owned(),
                    snapshot_token: preview.snapshot_token,
                    overwrite: false,
                },
                &fonts,
            )
            .expect("LaTeX export");
        let project = std::path::PathBuf::from(result.latex_directory.expect("project path"));
        for name in [
            "main.tex",
            "entries.tex",
            "reverse-index.tex",
            "README.md",
            ".latexmkrc",
        ] {
            assert!(project.join(name).is_file(), "missing {name}");
        }
        let main = std::fs::read_to_string(project.join("main.tex")).expect("main.tex");
        assert!(main.contains("Field \\#1 \\& notes"));
        assert!(main.contains("A\\_B"));
        assert!(main.contains("\\ovalbox{\\scriptsize\\bfseries 例}"));
        assert!(!main.contains("zxjatype"));
        let entries = std::fs::read_to_string(project.join("entries.tex")).expect("entries.tex");
        assert!(entries.contains("過\\_\\#\\%"));
        assert!(entries.contains("\\textbackslash{}test \\{value\\}"));
        assert!(entries.contains("\\BkuwMeta{[註] 詞條備註}"));
        assert!(entries.contains("kuo˥˩"));
        assert!(!entries.contains("IPA:"));
        assert!(!entries.contains("(他過河了。)"));
        let entry_position = entries.find("\\BkuwEntry").expect("entry heading");
        let note_position = entries.find("[註] 詞條備註").expect("entry note");
        let sense_position = entries.find("\\BkuwSense").expect("sense");
        assert!(entry_position < note_position && note_position < sense_position);
        let reverse = std::fs::read_to_string(project.join("reverse-index.tex")).expect("index");
        assert!(reverse.contains("\\pageref{entry:"));

        let zip_path = result.zip_path.expect("zip path");
        let file = std::fs::File::open(zip_path).expect("zip");
        let mut archive = zip::ZipArchive::new(file).expect("archive");
        let mut names = (0..archive.len())
            .map(|index| archive.by_index(index).expect("member").name().to_owned())
            .collect::<Vec<_>>();
        names.sort();
        assert!(names.contains(&"fonts/tex-gyre-termes/texgyretermes-regular.otf".into()));
        assert!(names.contains(&"fonts/tex-gyre-termes/LICENSE.txt".into()));
        assert!(names.contains(&"fonts/noto-serif-cjk-tc/NotoSerifCJKtc-Regular.otf".into()));
        assert!(names.contains(&"fonts/noto-serif-cjk-tc/LICENSE.txt".into()));
        assert!(names.contains(&"fonts/chiron-sung-hk/ChironSungHK-R.otf".into()));
        assert!(names.contains(&"fonts/chiron-sung-hk/ChironSungHK-B.otf".into()));
        assert!(names.contains(&"fonts/chiron-sung-hk/LICENSE.txt".into()));
        assert_eq!(result.pdf_status, crate::domain::PdfStatus::NotRequested);
    }

    #[test]
    fn latex_uses_project_order_and_renders_direct_incoming_root_entries() {
        let (directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let primary_id = snapshot.writing_systems[0].id.clone();
        session
            .save_entry_sort_settings(EntrySortSettingsV1 {
                version: 1,
                mode: EntrySortMode::Auto,
                writing_system_id: primary_id.clone(),
                alphabet: vec!["m".into(), "h".into()],
            })
            .expect("sort settings");

        let mut root = session.create_entry().expect("root");
        let root_id = root.id.clone();
        root.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: primary_id.clone(),
            text: "hako".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        root.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("橋".into()),
            definition: None,
            part_of_speech: None,
            semantic_domain: None,
            sort_order: 0,
            examples: vec![],
        });
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: root,
            })
            .expect("root save");

        for (form, gloss, relation_type) in [
            ("hako utux", "彩虹", "root"),
            ("mhako", "搭橋", "root"),
            ("bahako", "橋基", "base"),
        ] {
            let mut entry = session.create_entry().expect("related");
            entry.forms.push(EntryForm {
                id: super::new_id(),
                writing_system_id: primary_id.clone(),
                text: form.into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 0,
            });
            entry.senses.push(Sense {
                id: super::new_id(),
                gloss: Some(gloss.into()),
                definition: None,
                part_of_speech: None,
                semantic_domain: None,
                sort_order: 0,
                examples: vec![],
            });
            entry.relations.push(EntryRelation {
                id: super::new_id(),
                target_entry_id: Some(root_id.clone()),
                relation_type: relation_type.into(),
                fallback_text: None,
                notes: None,
                sort_order: 0,
            });
            if form == "mhako" {
                entry.relations.push(EntryRelation {
                    id: super::new_id(),
                    target_entry_id: Some(root_id.clone()),
                    relation_type: "base".into(),
                    fallback_text: None,
                    notes: None,
                    sort_order: 1,
                });
            }
            session
                .save_entry(SaveEntryRequest {
                    expected_revision: 0,
                    entry,
                })
                .expect("related save");
        }
        let mut settings = session.snapshot().expect("snapshot").export_settings;
        settings.latex.related_entries = RelatedEntriesMode::Root;
        session
            .save_export_settings(settings)
            .expect("export settings");
        let fonts = FontManager::seeded_for_tests(
            directory.path().join("font-cache"),
            &["tex-gyre-termes", "noto-serif"],
        );
        let preview = session
            .preview_export_with_fonts(ExportKind::Latex, &fonts)
            .expect("preview");
        let result = session
            .export_project_with_fonts(
                ExportProjectRequest {
                    kind: ExportKind::Latex,
                    destination: directory.path().to_string_lossy().into_owned(),
                    snapshot_token: preview.snapshot_token,
                    overwrite: false,
                },
                &fonts,
            )
            .expect("export");
        let entries = std::fs::read_to_string(
            std::path::PathBuf::from(result.latex_directory.expect("directory"))
                .join("entries.tex"),
        )
        .expect("entries");
        assert!(entries.find("mhako").expect("m entry") < entries.find("hako").expect("h entry"));
        assert!(entries.contains("\\BkuwSection{M}"));
        assert!(entries.contains("\\BkuwRelated{entry:"));
        let related = entries
            .split("\\BkuwRelatedStart")
            .nth(1)
            .expect("related block")
            .split("\\BkuwRelatedEnd")
            .next()
            .expect("related end");
        assert!(related.contains("hako utux"));
        assert!(related.contains("彩虹"));
        assert!(related.contains("mhako"));
        assert!(related.contains("搭橋"));
        assert!(!related.contains("bahako"));
        assert!(entries.contains("詞根：hako"));
        assert!(entries.contains("詞根：hako；詞基：hako"));

        let mut settings = session.snapshot().expect("snapshot").export_settings;
        settings.latex.related_entries = RelatedEntriesMode::Both;
        session
            .save_export_settings(settings)
            .expect("both settings");
        let preview = session
            .preview_export_with_fonts(ExportKind::Latex, &fonts)
            .expect("both preview");
        let result = session
            .export_project_with_fonts(
                ExportProjectRequest {
                    kind: ExportKind::Latex,
                    destination: directory.path().to_string_lossy().into_owned(),
                    snapshot_token: preview.snapshot_token,
                    overwrite: false,
                },
                &fonts,
            )
            .expect("both export");
        let entries = std::fs::read_to_string(
            std::path::PathBuf::from(result.latex_directory.expect("directory"))
                .join("entries.tex"),
        )
        .expect("both entries");
        let related = entries
            .split("\\BkuwRelatedStart")
            .nth(1)
            .expect("both related block")
            .split("\\BkuwRelatedEnd")
            .next()
            .expect("both related end");
        assert!(related.contains("bahako"));
        assert!(related.contains("橋基"));
        assert!(entries.contains("詞基：hako"));
    }

    #[test]
    fn latex_preview_fails_when_the_mandatory_termes_pack_is_missing() {
        let (directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let mut entry = session.create_entry().expect("entry");
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: snapshot.writing_systems[0].id.clone(),
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
            .expect("save");
        let fonts = FontManager::new(directory.path().join("font-cache"));

        let preview = session
            .preview_export_with_fonts(ExportKind::Latex, &fonts)
            .expect("preview");
        assert!(preview.has_errors());
        assert!(preview.issues.iter().any(|issue| {
            issue.code == "latex.font_pack_missing"
                && issue.details.as_deref() == Some("tex-gyre-termes")
        }));
        assert!(
            preview
                .required_font_packs
                .iter()
                .any(|pack| pack.id == "tex-gyre-termes")
        );
    }

    #[cfg(unix)]
    #[test]
    fn pdf_export_reports_created_missing_compile_failure_and_timeout() {
        use std::{os::unix::fs::PermissionsExt, time::Duration};

        let (directory, mut session) = create_session();
        let snapshot = session.snapshot().expect("snapshot");
        let writing_system_id = snapshot.writing_systems[0].id.clone();
        let mut entry = session.create_entry().expect("entry");
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: writing_system_id.clone(),
            text: "word".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        entry.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("詞".into()),
            definition: None,
            part_of_speech: None,
            semantic_domain: None,
            sort_order: 0,
            examples: vec![],
        });
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect("save");
        let fonts = FontManager::seeded_for_tests(
            directory.path().join("font-cache"),
            &["tex-gyre-termes", "noto-serif"],
        );
        let preview = session
            .preview_export_with_fonts(ExportKind::Pdf, &fonts)
            .expect("preview");
        let export = |session: &ProjectSession| {
            session.export_project_with_fonts(
                ExportProjectRequest {
                    kind: ExportKind::Pdf,
                    destination: directory.path().to_string_lossy().into_owned(),
                    snapshot_token: preview.snapshot_token.clone(),
                    overwrite: false,
                },
                &fonts,
            )
        };

        crate::export::with_test_xelatex(None, Duration::from_secs(1), || {
            let result = export(&session).expect("source export succeeds without XeLaTeX");
            assert_eq!(result.pdf_status, crate::domain::PdfStatus::XeLatexMissing);
            assert!(result.pdf_path.is_none());
        });

        let success = directory.path().join("xelatex-success");
        std::fs::write(&success, "#!/bin/sh\ntouch main.pdf\n").expect("success script");
        std::fs::set_permissions(&success, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
        crate::export::with_test_xelatex(Some(success), Duration::from_secs(1), || {
            let result = export(&session).expect("PDF export");
            assert_eq!(result.pdf_status, crate::domain::PdfStatus::Created);
            assert!(
                result
                    .pdf_path
                    .as_ref()
                    .is_some_and(|path| std::path::Path::new(path).is_file())
            );
        });

        let failure = directory.path().join("xelatex-failure");
        std::fs::write(&failure, "#!/bin/sh\necho compile-failed >&2\nexit 7\n")
            .expect("failure script");
        std::fs::set_permissions(&failure, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
        crate::export::with_test_xelatex(Some(failure), Duration::from_secs(1), || {
            let error = export(&session).expect_err("compile failure");
            assert_eq!(error.code, "latex_compile");
            let diagnostic = error.details.expect("diagnostic path");
            assert!(std::path::Path::new(&diagnostic).is_file());
            assert!(
                std::fs::read_to_string(diagnostic)
                    .expect("diagnostic")
                    .contains("compile-failed")
            );
        });

        let timeout = directory.path().join("xelatex-timeout");
        std::fs::write(&timeout, "#!/bin/sh\nsleep 2\n").expect("timeout script");
        std::fs::set_permissions(&timeout, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
        crate::export::with_test_xelatex(Some(timeout), Duration::from_millis(25), || {
            let error = export(&session).expect_err("compile timeout");
            assert_eq!(error.code, "latex_timeout");
            assert!(std::path::Path::new(&error.details.expect("diagnostic path")).is_file());
        });
    }

    #[test]
    #[ignore = "requires a local XeLaTeX installation; run in the portable-template CI job"]
    fn portable_xelatex_template_compiles_with_the_real_engine() {
        if !crate::export::detect_xelatex().available {
            return;
        }
        let (directory, mut session) = create_session();
        let mut snapshot = session.snapshot().expect("snapshot");
        snapshot.writing_systems[0].script_code = Some("Hant".into());
        let ipa_id = super::new_id();
        let mut writing_systems = snapshot.writing_systems.clone();
        writing_systems.push(WritingSystem {
            id: ipa_id.clone(),
            name: "IPA".into(),
            kind: "phonetic".into(),
            script_code: Some("Latn".into()),
            language_tag: None,
            display_role: None,
            sort_order: 1,
            font_family: None,
            notes: None,
        });
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name.clone(),
                language_name: snapshot.project.language_name.clone(),
                language_code: snapshot.project.language_code.clone(),
                analysis_language: Some("zh-TW".into()),
                description: snapshot.project.description.clone(),
                writing_systems,
                part_of_speech_options: vec!["verb".into()],
                semantic_domain_options: vec![],
            })
            .expect("Hant settings");
        let writing_system_id = snapshot.writing_systems[0].id.clone();
        let mut entry = session.create_entry().expect("entry");
        let root_id = entry.id.clone();
        entry.notes = Some("詞條註解".into());
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: writing_system_id.clone(),
            text: "過".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        entry.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id: ipa_id.clone(),
            text: "kuo˥˩".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 1,
        });
        entry.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("通過".into()),
            definition: Some("cross".into()),
            part_of_speech: Some("verb".into()),
            semantic_domain: None,
            sort_order: 0,
            examples: vec![Example {
                id: super::new_id(),
                translation: Some("他過河了。".into()),
                notes: None,
                sort_order: 0,
                forms: vec![ExampleForm {
                    id: super::new_id(),
                    writing_system_id: writing_system_id.clone(),
                    text: "他過河了。".into(),
                    sort_order: 0,
                }],
            }],
        });
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry,
            })
            .expect("save");
        let mut related = session.create_entry().expect("related entry");
        related.forms.push(EntryForm {
            id: super::new_id(),
            writing_system_id,
            text: "經過".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        related.senses.push(Sense {
            id: super::new_id(),
            gloss: Some("通行".into()),
            definition: None,
            part_of_speech: Some("verb".into()),
            semantic_domain: None,
            sort_order: 0,
            examples: vec![],
        });
        related.relations.push(EntryRelation {
            id: super::new_id(),
            target_entry_id: Some(root_id),
            relation_type: "root".into(),
            fallback_text: None,
            notes: None,
            sort_order: 0,
        });
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: related,
            })
            .expect("save related entry");
        let mut export_settings = session.snapshot().expect("snapshot").export_settings;
        export_settings.latex.related_entries = RelatedEntriesMode::Root;
        export_settings.latex.pronunciation_writing_system_id = Some(ipa_id);
        export_settings.latex.font_presets.insert(
            snapshot.writing_systems[0].id.clone(),
            FontPreset::ChironHeiHk,
        );
        session
            .save_export_settings(export_settings)
            .expect("related entries setting");
        let fonts = FontManager::new(directory.path().join("font-cache"));
        fonts
            .install("tex-gyre-termes")
            .expect("install TeX Gyre Termes");
        fonts
            .install("noto-serif-cjk-tc")
            .expect("install Noto Serif CJK TC");
        fonts.install("charis-sil").expect("install Charis SIL");
        fonts
            .install("chiron-hei-hk")
            .expect("install Chiron Hei HK");
        let preview = session
            .preview_export_with_fonts(ExportKind::Pdf, &fonts)
            .expect("preview");
        let destination = std::env::var("BKUW_LATEX_SMOKE_DESTINATION")
            .unwrap_or_else(|_| directory.path().to_string_lossy().into_owned());
        let result = session
            .export_project_with_fonts(
                ExportProjectRequest {
                    kind: ExportKind::Pdf,
                    destination,
                    snapshot_token: preview.snapshot_token,
                    overwrite: false,
                },
                &fonts,
            )
            .unwrap_or_else(|error| {
                let diagnostic = error
                    .details
                    .as_deref()
                    .and_then(|path| std::fs::read_to_string(path).ok());
                panic!("real XeLaTeX export failed: {error:?}\n{diagnostic:?}");
            });
        assert_eq!(result.pdf_status, crate::domain::PdfStatus::Created);
        assert!(
            result
                .pdf_path
                .is_some_and(|path| std::path::Path::new(&path).is_file())
        );
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
                analysis_language: None,
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
                analysis_language: None,
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
                analysis_language: None,
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
