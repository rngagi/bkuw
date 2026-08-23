use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};

use csv::ReaderBuilder;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    database::ProjectSession,
    domain::{
        CorpusExportSettings, CorpusPartOfSpeech, CreateProjectFromCsvRequest,
        CreateProjectRequest, CsvColumn, CsvColumnMapping, CsvDelimiter, CsvImportGroup,
        CsvImportPreview, CsvImportResult, CsvInspection, CsvMappingTarget, CsvPreviewGroup,
        CsvPreviewIssue, CsvPreviewIssueSeverity, CsvPreviewRequest, EntryForm, EntryRelation,
        Example, ExampleForm, ExportSettingsV1, FontPreset, LatexExportSettings, LexicalEntry,
        RelatedEntriesMode, ReverseIndexMode, SectionMode, Sense, WritingSystem,
    },
    error::{AppError, AppResult},
    search::normalize_text,
};

const RNGAGI_HEADERS: [&str; 9] = [
    "form",
    "gloss_zh",
    "word_root",
    "example",
    "example_translation_zh",
    "ipa",
    "part_of_speech",
    "gloss_en",
    "notes",
];

struct ParsedCsv {
    hash: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    profile: Option<String>,
}

#[derive(Clone, Default, PartialEq, Eq)]
struct ImportRow {
    entry_forms: HashMap<String, String>,
    entry_notes: Option<String>,
    gloss: Option<String>,
    definition: Option<String>,
    part_of_speech: Option<String>,
    semantic_domain: Option<String>,
    example_forms: HashMap<String, String>,
    translation: Option<String>,
    example_notes: Option<String>,
    roots: Vec<String>,
}

struct PreparedImport {
    parsed: ParsedCsv,
    rows: Vec<ImportRow>,
    groups: Vec<CsvImportGroup>,
    issues: Vec<CsvPreviewIssue>,
    primary_writing_system_id: Option<String>,
}

pub(crate) fn inspect(path: &str, delimiter: Option<CsvDelimiter>) -> AppResult<CsvInspection> {
    let bytes = read_source(path)?;
    let delimiter = delimiter.unwrap_or_else(|| detect_delimiter(&bytes));
    let parsed = parse(&bytes, delimiter)?;
    let columns = parsed
        .headers
        .iter()
        .enumerate()
        .map(|(index, name)| CsvColumn {
            index,
            name: name.clone(),
            samples: parsed
                .rows
                .iter()
                .filter_map(|row| row.get(index))
                .filter(|value| !value.trim().is_empty())
                .take(3)
                .cloned()
                .collect(),
        })
        .collect();
    Ok(CsvInspection {
        source_path: path.to_owned(),
        file_name: Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("data.csv")
            .to_owned(),
        sha256: parsed.hash,
        delimiter,
        row_count: parsed.rows.len(),
        columns,
        profile: parsed.profile,
    })
}

pub(crate) fn preview(request: &CsvPreviewRequest) -> AppResult<CsvImportPreview> {
    let prepared = prepare(request)?;
    Ok(preview_from_prepared(request, &prepared))
}

pub(crate) fn create(
    request: CreateProjectFromCsvRequest,
) -> AppResult<(ProjectSession, CsvImportResult)> {
    let prepared = prepare(&request.preview)?;
    let current = preview_from_prepared(&request.preview, &prepared);
    if current.preview_token != request.preview_token {
        return Err(AppError::new(
            "stale_preview",
            "The CSV or import configuration changed after preview.",
        ));
    }
    if current.blocking_error_count > 0 {
        return Err(AppError::new(
            "csv_validation",
            "Resolve all blocking CSV issues before importing.",
        ));
    }
    let (systems, id_map) = final_writing_systems(&request.preview.project.writing_systems);
    let entries = build_entries(&prepared, &request.preview, &id_map);
    let pos_options = unique_values(entries.iter().flat_map(|entry| {
        entry
            .senses
            .iter()
            .filter_map(|sense| sense.part_of_speech.clone())
    }));
    let semantic_options = unique_values(entries.iter().flat_map(|entry| {
        entry
            .senses
            .iter()
            .filter_map(|sense| sense.semantic_domain.clone())
    }));
    let primary_id = systems
        .iter()
        .find(|system| system.display_role.as_deref() == Some("primary"))
        .map(|system| system.id.clone())
        .ok_or_else(|| AppError::new("csv_validation", "A primary writing system is required."))?;
    let pronunciation_id = systems
        .iter()
        .find(|system| matches!(system.kind.as_str(), "phonetic" | "phonemic"))
        .map(|system| system.id.clone());
    let pos_mappings = pos_options
        .iter()
        .filter_map(|value| known_pos(value).map(|mapped| (value.clone(), mapped)))
        .collect();
    let font_presets = systems
        .iter()
        .map(|system| (system.id.clone(), FontPreset::Auto))
        .collect();
    let export_settings = ExportSettingsV1 {
        version: 1,
        corpus: CorpusExportSettings {
            part_of_speech_mappings: pos_mappings,
        },
        latex: LatexExportSettings {
            title: normalize_text(request.preview.project.name.trim()),
            author: String::new(),
            headword_writing_system_id: primary_id.clone(),
            pronunciation_writing_system_id: pronunciation_id,
            example_writing_system_id: primary_id,
            collation_language_tag: None,
            section_mode: SectionMode::Auto,
            reverse_index: ReverseIndexMode::Gloss,
            related_entries: RelatedEntriesMode::None,
            include_sense_images: false,
            include_semantic_domains: true,
            font_presets,
        },
    };
    let session = ProjectSession::create_imported(
        CreateProjectRequest {
            parent_dir: request.preview.project.parent_dir.clone(),
            name: request.preview.project.name.clone(),
            language_name: request.preview.project.language_name.clone(),
            language_code: request.preview.project.language_code.clone(),
        },
        request.preview.project.analysis_language.clone(),
        systems,
        pos_options,
        semantic_options,
        export_settings,
        entries,
    )?;
    let snapshot = session.snapshot()?;
    let result = CsvImportResult {
        snapshot,
        imported_entry_count: current.import_entry_count,
        imported_sense_count: current.import_sense_count,
        skipped_row_count: current.skipped_row_count,
        warnings: current
            .issues
            .into_iter()
            .filter(|issue| issue.severity == CsvPreviewIssueSeverity::Warning)
            .collect(),
    };
    Ok((session, result))
}

fn read_source(path: &str) -> AppResult<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::with_details(
            "csv_file",
            "The CSV file is unavailable.",
            error.to_string(),
        )
    })?;
    if !metadata.is_file() {
        return Err(AppError::new("csv_file", "Select a CSV file."));
    }
    fs::read(path).map_err(Into::into)
}

fn delimiter_byte(delimiter: CsvDelimiter) -> u8 {
    match delimiter {
        CsvDelimiter::Comma => b',',
        CsvDelimiter::Tab => b'\t',
        CsvDelimiter::Semicolon => b';',
    }
}

fn detect_delimiter(bytes: &[u8]) -> CsvDelimiter {
    [
        CsvDelimiter::Comma,
        CsvDelimiter::Tab,
        CsvDelimiter::Semicolon,
    ]
    .into_iter()
    .max_by_key(|candidate| {
        let mut reader = ReaderBuilder::new()
            .delimiter(delimiter_byte(*candidate))
            .has_headers(true)
            .flexible(false)
            .from_reader(bytes);
        let columns = reader.headers().map_or(0, csv::StringRecord::len);
        let valid_rows = reader.records().take(12).filter(Result::is_ok).count();
        (usize::from(columns > 1), valid_rows, columns)
    })
    .unwrap_or(CsvDelimiter::Comma)
}

fn parse(bytes: &[u8], delimiter: CsvDelimiter) -> AppResult<ParsedCsv> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        AppError::with_details(
            "csv_encoding",
            "CSV import supports UTF-8 and UTF-8 BOM only.",
            error.to_string(),
        )
    })?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter_byte(delimiter))
        .has_headers(true)
        .flexible(false)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(csv_error)?
        .iter()
        .map(|value| normalize_text(value.trim()))
        .collect::<Vec<_>>();
    if headers.is_empty() || headers.iter().any(String::is_empty) {
        return Err(AppError::new(
            "csv_headers",
            "CSV import requires a non-empty header row.",
        ));
    }
    let mut seen = HashSet::new();
    if headers.iter().any(|header| !seen.insert(header.clone())) {
        return Err(AppError::new(
            "csv_headers",
            "CSV header names must be unique.",
        ));
    }
    let rows = reader
        .records()
        .map(|record| {
            record
                .map(|record| record.iter().map(normalize_text).collect())
                .map_err(csv_error)
        })
        .collect::<AppResult<Vec<Vec<String>>>>()?;
    let profile = (headers.iter().map(String::as_str).eq(RNGAGI_HEADERS))
        .then(|| "rngagi-corpus-v0.3".into());
    Ok(ParsedCsv {
        hash: hex::encode(Sha256::digest(bytes)),
        headers,
        rows,
        profile,
    })
}

fn csv_error(error: csv::Error) -> AppError {
    AppError::with_details(
        "csv_parse",
        "The CSV file could not be parsed.",
        error.to_string(),
    )
}

fn prepare(request: &CsvPreviewRequest) -> AppResult<PreparedImport> {
    let bytes = read_source(&request.source_path)?;
    let parsed = parse(&bytes, request.delimiter)?;
    let mut issues = Vec::new();
    let primary_systems = request
        .project
        .writing_systems
        .iter()
        .filter(|system| system.display_role.as_deref() == Some("primary"))
        .collect::<Vec<_>>();
    if primary_systems.len() != 1 {
        issues.push(global_error("primary_writing_system"));
    }
    if request.project.name.trim().is_empty() || request.project.parent_dir.trim().is_empty() {
        issues.push(global_error("project_required"));
    }
    if request
        .project
        .writing_systems
        .iter()
        .any(|system| system.name.trim().is_empty())
    {
        issues.push(global_error("writing_system_required"));
    }
    let primary_writing_system_id = primary_systems.first().map(|system| system.id.clone());
    validate_mappings(
        request,
        &parsed,
        primary_writing_system_id.as_deref(),
        &mut issues,
    );
    let root_delimiter = request.root_delimiter.chars().next().unwrap_or(';');
    let rows = parsed
        .rows
        .iter()
        .enumerate()
        .map(|(index, values)| {
            row_data(index, values, &parsed, request, root_delimiter, &mut issues)
        })
        .collect::<Vec<_>>();
    let excluded = request
        .excluded_rows
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let groups = resolve_groups(
        request,
        &rows,
        primary_writing_system_id.as_deref(),
        &excluded,
        &mut issues,
    );
    validate_group_conflicts(&groups, &rows, &mut issues);
    Ok(PreparedImport {
        parsed,
        rows,
        groups,
        issues,
        primary_writing_system_id,
    })
}

fn validate_mappings(
    request: &CsvPreviewRequest,
    parsed: &ParsedCsv,
    primary_id: Option<&str>,
    issues: &mut Vec<CsvPreviewIssue>,
) {
    let systems = request
        .project
        .writing_systems
        .iter()
        .map(|system| system.id.as_str())
        .collect::<HashSet<_>>();
    let mut columns = HashSet::new();
    let mut targets = HashSet::new();
    let mut primary_count = 0;
    for mapping in &request.mappings {
        if mapping.column_index >= parsed.headers.len() || !columns.insert(mapping.column_index) {
            issues.push(global_error("mapping_column_duplicate"));
        }
        let target_key = target_key(&mapping.target);
        if !matches!(mapping.target, CsvMappingTarget::Ignore) && !targets.insert(target_key) {
            issues.push(global_error("mapping_target_duplicate"));
        }
        match &mapping.target {
            CsvMappingTarget::EntryForm { writing_system_id }
            | CsvMappingTarget::ExampleForm { writing_system_id }
                if !systems.contains(writing_system_id.as_str()) =>
            {
                issues.push(global_error("mapping_writing_system_missing"));
            }
            _ => {}
        }
        if matches!(&mapping.target, CsvMappingTarget::EntryForm { writing_system_id } if Some(writing_system_id.as_str()) == primary_id)
        {
            primary_count += 1;
        }
    }
    if primary_count != 1 {
        issues.push(global_error("primary_form_mapping"));
    }
}

fn target_key(target: &CsvMappingTarget) -> String {
    match target {
        CsvMappingTarget::Ignore => "ignore".into(),
        CsvMappingTarget::EntryForm { writing_system_id } => {
            format!("entryForm:{writing_system_id}")
        }
        CsvMappingTarget::EntryNotes => "entryNotes".into(),
        CsvMappingTarget::SenseGloss => "senseGloss".into(),
        CsvMappingTarget::SenseDefinition => "senseDefinition".into(),
        CsvMappingTarget::PartOfSpeech => "partOfSpeech".into(),
        CsvMappingTarget::SemanticDomain => "semanticDomain".into(),
        CsvMappingTarget::ExampleForm { writing_system_id } => {
            format!("exampleForm:{writing_system_id}")
        }
        CsvMappingTarget::ExampleTranslation => "exampleTranslation".into(),
        CsvMappingTarget::ExampleNotes => "exampleNotes".into(),
        CsvMappingTarget::RootFallback => "rootFallback".into(),
    }
}

fn row_data(
    index: usize,
    values: &[String],
    parsed: &ParsedCsv,
    request: &CsvPreviewRequest,
    root_delimiter: char,
    issues: &mut Vec<CsvPreviewIssue>,
) -> ImportRow {
    let mut row = ImportRow::default();
    for CsvColumnMapping {
        column_index,
        target,
    } in &request.mappings
    {
        let value = values
            .get(*column_index)
            .map(String::as_str)
            .unwrap_or_default()
            .trim();
        if value.is_empty() || matches!(target, CsvMappingTarget::Ignore) {
            continue;
        }
        let normalized = normalize_text(value);
        match target {
            CsvMappingTarget::Ignore => {}
            CsvMappingTarget::EntryForm { writing_system_id } => {
                row.entry_forms
                    .insert(writing_system_id.clone(), normalized);
            }
            CsvMappingTarget::EntryNotes => {
                if parsed.profile.as_deref() == Some("rngagi-corpus-v0.3")
                    && parsed
                        .headers
                        .get(*column_index)
                        .is_some_and(|header| header == "notes")
                {
                    parse_rngagi_notes(&normalized, &mut row, index, issues);
                } else {
                    row.entry_notes = Some(normalized);
                }
            }
            CsvMappingTarget::SenseGloss => row.gloss = Some(normalized),
            CsvMappingTarget::SenseDefinition => append_text(&mut row.definition, &normalized),
            CsvMappingTarget::PartOfSpeech => row.part_of_speech = Some(normalized),
            CsvMappingTarget::SemanticDomain => row.semantic_domain = Some(normalized),
            CsvMappingTarget::ExampleForm { writing_system_id } => {
                row.example_forms
                    .insert(writing_system_id.clone(), normalized);
            }
            CsvMappingTarget::ExampleTranslation => row.translation = Some(normalized),
            CsvMappingTarget::ExampleNotes => row.example_notes = Some(normalized),
            CsvMappingTarget::RootFallback => {
                row.roots = normalized
                    .split(root_delimiter)
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(normalize_text)
                    .collect();
            }
        }
    }
    row
}

fn parse_rngagi_notes(
    value: &str,
    row: &mut ImportRow,
    index: usize,
    issues: &mut Vec<CsvPreviewIssue>,
) {
    let mut unknown = Vec::new();
    for line in value.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((key, content)) = line.split_once(':') else {
            unknown.push(line.to_owned());
            continue;
        };
        let content = content.trim();
        match key.trim() {
            "entry_notes" => append_text(&mut row.entry_notes, content),
            "sense_definition" => append_text(&mut row.definition, content),
            "semantic_domain" => row.semantic_domain = nonempty(content),
            "example_notes" => append_text(&mut row.example_notes, content),
            _ => unknown.push(line.to_owned()),
        }
    }
    if !unknown.is_empty() {
        append_text(&mut row.definition, &unknown.join("\n"));
        issues.push(CsvPreviewIssue {
            severity: CsvPreviewIssueSeverity::Warning,
            code: "unknown_rngagi_notes".into(),
            row_indices: vec![index],
            details: None,
        });
    }
}

fn resolve_groups(
    request: &CsvPreviewRequest,
    rows: &[ImportRow],
    primary_id: Option<&str>,
    excluded: &HashSet<usize>,
    issues: &mut Vec<CsvPreviewIssue>,
) -> Vec<CsvImportGroup> {
    let included = (0..rows.len())
        .filter(|index| !excluded.contains(index))
        .collect::<Vec<_>>();
    for index in &included {
        let primary = primary_id.and_then(|id| rows[*index].entry_forms.get(id));
        if primary.is_none_or(|value| value.trim().is_empty()) {
            issues.push(CsvPreviewIssue {
                severity: CsvPreviewIssueSeverity::Error,
                code: "primary_form_required".into(),
                row_indices: vec![*index],
                details: None,
            });
        }
    }
    if request.groups.is_empty() {
        let mut groups: Vec<CsvImportGroup> = Vec::new();
        for index in included {
            let primary = primary_id.and_then(|id| rows[index].entry_forms.get(id));
            let joins_previous = groups
                .last()
                .and_then(|group| group.row_indices.last())
                .is_some_and(|previous| {
                    *previous + 1 == index
                        && primary_id.and_then(|id| rows[*previous].entry_forms.get(id)) == primary
                });
            if joins_previous {
                groups
                    .last_mut()
                    .expect("group exists")
                    .row_indices
                    .push(index);
            } else {
                groups.push(CsvImportGroup {
                    row_indices: vec![index],
                });
            }
        }
        return groups;
    }
    let mut covered = HashSet::new();
    let mut valid = true;
    for group in &request.groups {
        if group.row_indices.is_empty()
            || group
                .row_indices
                .windows(2)
                .any(|pair| pair[0] + 1 != pair[1])
            || group.row_indices.iter().any(|index| {
                *index >= rows.len() || excluded.contains(index) || !covered.insert(*index)
            })
        {
            valid = false;
        }
    }
    if covered.len() != included.len() {
        valid = false;
    }
    if valid {
        let mut groups = request.groups.clone();
        groups.sort_by_key(|group| group.row_indices[0]);
        groups
    } else {
        issues.push(global_error("groups_invalid"));
        included
            .into_iter()
            .map(|index| CsvImportGroup {
                row_indices: vec![index],
            })
            .collect()
    }
}

fn validate_group_conflicts(
    groups: &[CsvImportGroup],
    rows: &[ImportRow],
    issues: &mut Vec<CsvPreviewIssue>,
) {
    for group in groups {
        let distinct_forms = group_values(group, rows, |row| {
            format!("{:?}", sorted_map(&row.entry_forms))
        });
        let distinct_notes = group_values(group, rows, |row| {
            row.entry_notes.clone().unwrap_or_default()
        });
        let distinct_roots = group_values(group, rows, |row| row.roots.join("\u{1f}"));
        if distinct_forms > 1 || distinct_notes > 1 || distinct_roots > 1 {
            issues.push(CsvPreviewIssue {
                severity: CsvPreviewIssueSeverity::Error,
                code: "group_entry_conflict".into(),
                row_indices: group.row_indices.clone(),
                details: None,
            });
        }
    }
}

fn sorted_map(map: &HashMap<String, String>) -> BTreeMap<&str, &str> {
    map.iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
}

fn group_values<F>(group: &CsvImportGroup, rows: &[ImportRow], value: F) -> usize
where
    F: Fn(&ImportRow) -> String,
{
    group
        .row_indices
        .iter()
        .map(|index| value(&rows[*index]))
        .collect::<HashSet<_>>()
        .len()
}

fn preview_from_prepared(
    request: &CsvPreviewRequest,
    prepared: &PreparedImport,
) -> CsvImportPreview {
    let token = preview_token(&prepared.parsed.hash, request);
    let blocking_error_count = prepared
        .issues
        .iter()
        .filter(|issue| issue.severity == CsvPreviewIssueSeverity::Error)
        .count();
    let warning_count = prepared
        .issues
        .iter()
        .filter(|issue| issue.severity == CsvPreviewIssueSeverity::Warning)
        .count();
    let blocked_rows = prepared
        .issues
        .iter()
        .filter(|issue| issue.severity == CsvPreviewIssueSeverity::Error)
        .flat_map(|issue| issue.row_indices.iter().copied())
        .collect::<HashSet<_>>();
    let groups = prepared
        .groups
        .iter()
        .map(|group| CsvPreviewGroup {
            row_indices: group.row_indices.clone(),
            primary_form: group
                .row_indices
                .first()
                .and_then(|index| {
                    prepared
                        .primary_writing_system_id
                        .as_deref()
                        .and_then(|id| prepared.rows[*index].entry_forms.get(id))
                })
                .cloned()
                .unwrap_or_default(),
            blocked: group
                .row_indices
                .iter()
                .any(|index| blocked_rows.contains(index)),
        })
        .collect::<Vec<_>>();
    CsvImportPreview {
        preview_token: token,
        source_row_count: prepared.rows.len(),
        import_entry_count: groups.len(),
        import_sense_count: groups.iter().map(|group| group.row_indices.len()).sum(),
        skipped_row_count: request
            .excluded_rows
            .iter()
            .copied()
            .filter(|index| *index < prepared.rows.len())
            .collect::<HashSet<_>>()
            .len(),
        blocking_error_count,
        warning_count,
        issues: prepared.issues.clone(),
        groups,
    }
}

fn preview_token(file_hash: &str, request: &CsvPreviewRequest) -> String {
    let mut hash = Sha256::new();
    hash.update(file_hash.as_bytes());
    hash.update(serde_json::to_vec(request).unwrap_or_default());
    hex::encode(hash.finalize())
}

fn build_entries(
    prepared: &PreparedImport,
    request: &CsvPreviewRequest,
    ids: &HashMap<String, String>,
) -> Vec<LexicalEntry> {
    prepared
        .groups
        .iter()
        .map(|group| {
            let first = &prepared.rows[group.row_indices[0]];
            let entry_id = new_id();
            let forms = request
                .project
                .writing_systems
                .iter()
                .filter_map(|system| {
                    first
                        .entry_forms
                        .get(&system.id)
                        .map(|text| (&system.id, text))
                })
                .filter(|(_, text)| !text.trim().is_empty())
                .enumerate()
                .map(|(sort_order, (system_id, text))| EntryForm {
                    id: new_id(),
                    writing_system_id: ids
                        .get(system_id)
                        .cloned()
                        .unwrap_or_else(|| system_id.clone()),
                    text: normalize_text(text),
                    variant_label: None,
                    dialect: None,
                    status: None,
                    notes: None,
                    sort_order: sort_order as i64,
                })
                .collect();
            let senses = group
                .row_indices
                .iter()
                .enumerate()
                .map(|(sense_order, row_index)| {
                    let row = &prepared.rows[*row_index];
                    let has_example = !row.example_forms.is_empty()
                        || row.translation.is_some()
                        || row.example_notes.is_some();
                    let examples = has_example
                        .then(|| Example {
                            id: new_id(),
                            translation: normalized_option(row.translation.clone()),
                            notes: normalized_option(row.example_notes.clone()),
                            sort_order: 0,
                            forms: request
                                .project
                                .writing_systems
                                .iter()
                                .filter_map(|system| {
                                    row.example_forms
                                        .get(&system.id)
                                        .map(|text| (&system.id, text))
                                })
                                .filter(|(_, text)| !text.trim().is_empty())
                                .enumerate()
                                .map(|(sort_order, (system_id, text))| ExampleForm {
                                    id: new_id(),
                                    writing_system_id: ids
                                        .get(system_id)
                                        .cloned()
                                        .unwrap_or_else(|| system_id.clone()),
                                    text: normalize_text(text),
                                    sort_order: sort_order as i64,
                                })
                                .collect(),
                        })
                        .into_iter()
                        .collect();
                    Sense {
                        id: new_id(),
                        gloss: normalized_option(row.gloss.clone()),
                        definition: normalized_option(row.definition.clone()),
                        part_of_speech: normalized_option(row.part_of_speech.clone()),
                        semantic_domain: normalized_option(row.semantic_domain.clone()),
                        sort_order: sense_order as i64,
                        examples,
                    }
                })
                .collect();
            let relations = first
                .roots
                .iter()
                .enumerate()
                .map(|(sort_order, root)| EntryRelation {
                    id: new_id(),
                    target_entry_id: None,
                    relation_type: "root".into(),
                    fallback_text: Some(normalize_text(root)),
                    notes: None,
                    sort_order: sort_order as i64,
                })
                .collect();
            LexicalEntry {
                id: entry_id,
                notes: normalized_option(first.entry_notes.clone()),
                section_override: None,
                revision: 0,
                created_at: String::new(),
                updated_at: String::new(),
                forms,
                senses,
                relations,
            }
        })
        .collect()
}

fn final_writing_systems(
    source: &[WritingSystem],
) -> (Vec<WritingSystem>, HashMap<String, String>) {
    let id_map = source
        .iter()
        .map(|system| (system.id.clone(), new_id()))
        .collect::<HashMap<_, _>>();
    let systems = source
        .iter()
        .enumerate()
        .map(|(index, system)| WritingSystem {
            id: id_map[&system.id].clone(),
            name: normalize_text(system.name.trim()),
            kind: system.kind.clone(),
            script_code: normalized_option(system.script_code.clone()),
            language_tag: normalized_option(system.language_tag.clone()),
            display_role: system.display_role.clone(),
            sort_order: index as i64,
            font_family: normalized_option(system.font_family.clone()),
            notes: normalized_option(system.notes.clone()),
        })
        .collect();
    (systems, id_map)
}

fn unique_values(values: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values.filter(|value| seen.insert(value.clone())).collect()
}

fn known_pos(value: &str) -> Option<CorpusPartOfSpeech> {
    match value.trim().to_lowercase().as_str() {
        "n" | "noun" => Some(CorpusPartOfSpeech::Noun),
        "v" | "verb" => Some(CorpusPartOfSpeech::Verb),
        "adj" | "adjective" => Some(CorpusPartOfSpeech::Adjective),
        "adv" | "adverb" => Some(CorpusPartOfSpeech::Adverb),
        "pron" | "pronoun" => Some(CorpusPartOfSpeech::Pronoun),
        "part" | "particle" => Some(CorpusPartOfSpeech::Particle),
        "other" => Some(CorpusPartOfSpeech::Other),
        _ => None,
    }
}

fn append_text(target: &mut Option<String>, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    match target {
        Some(current) if !current.is_empty() => {
            current.push('\n');
            current.push_str(value.trim());
        }
        _ => *target = Some(normalize_text(value.trim())),
    }
}

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| normalize_text(value.trim()))
}
fn normalized_option(value: Option<String>) -> Option<String> {
    value.and_then(|value| nonempty(&value))
}
fn new_id() -> String {
    Uuid::new_v4().to_string()
}
fn global_error(code: &str) -> CsvPreviewIssue {
    CsvPreviewIssue {
        severity: CsvPreviewIssueSeverity::Error,
        code: code.into(),
        row_indices: Vec::new(),
        details: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_fixture(bytes: &[u8]) -> (tempfile::TempDir, String) {
        let directory = tempdir().expect("temp directory");
        let path = directory.path().join("input.csv");
        fs::write(&path, bytes).expect("fixture");
        (directory, path.to_string_lossy().into_owned())
    }

    fn request(path: String) -> CsvPreviewRequest {
        CsvPreviewRequest {
            source_path: path,
            delimiter: CsvDelimiter::Comma,
            project: crate::domain::CsvProjectSpec {
                parent_dir: "/tmp".into(),
                name: "Import".into(),
                language_name: None,
                language_code: None,
                analysis_language: Some("zh-TW".into()),
                writing_systems: vec![WritingSystem {
                    id: "primary".into(),
                    name: "Primary".into(),
                    kind: "orthography".into(),
                    script_code: None,
                    language_tag: None,
                    display_role: Some("primary".into()),
                    sort_order: 0,
                    font_family: None,
                    notes: None,
                }],
            },
            mappings: vec![
                CsvColumnMapping {
                    column_index: 0,
                    target: CsvMappingTarget::EntryForm {
                        writing_system_id: "primary".into(),
                    },
                },
                CsvColumnMapping {
                    column_index: 1,
                    target: CsvMappingTarget::SenseGloss,
                },
            ],
            groups: Vec::new(),
            excluded_rows: Vec::new(),
            root_delimiter: ";".into(),
        }
    }

    #[test]
    fn inspection_accepts_bom_and_detects_tab() {
        let (_directory, path) = write_fixture(b"\xef\xbb\xbfform\tgloss\na\tA\n");
        let result = inspect(&path, None).expect("inspection");
        assert_eq!(result.delimiter, CsvDelimiter::Tab);
        assert_eq!(result.columns[0].name, "form");
        assert_eq!(result.row_count, 1);
    }

    #[test]
    fn preview_groups_adjacent_forms_and_detects_entry_conflicts() {
        let (_directory, path) = write_fixture(b"form,gloss,notes\na,A,one\na,B,two\nb,C,three\n");
        let mut request = request(path);
        request.mappings.push(CsvColumnMapping {
            column_index: 2,
            target: CsvMappingTarget::EntryNotes,
        });
        let preview = preview(&request).expect("preview");
        assert_eq!(
            preview
                .groups
                .iter()
                .map(|group| group.row_indices.clone())
                .collect::<Vec<_>>(),
            vec![vec![0, 1], vec![2]]
        );
        assert!(
            preview
                .issues
                .iter()
                .any(|issue| issue.code == "group_entry_conflict")
        );
    }

    #[test]
    fn stale_token_is_rejected_before_project_creation() {
        let (directory, path) = write_fixture(b"form,gloss\na,A\n");
        let mut request = request(path.clone());
        request.project.parent_dir = directory.path().to_string_lossy().into_owned();
        let error = match create(CreateProjectFromCsvRequest {
            preview: request,
            preview_token: "stale".into(),
        }) {
            Ok(_) => panic!("stale token was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.code, "stale_preview");
    }

    #[test]
    fn rngagi_notes_preserve_unknown_content_in_definition() {
        let mut row = ImportRow::default();
        let mut issues = Vec::new();
        parse_rngagi_notes(
            "entry_notes: note\nsense_definition: def\ncustom: keep",
            &mut row,
            0,
            &mut issues,
        );
        assert_eq!(row.entry_notes.as_deref(), Some("note"));
        assert_eq!(row.definition.as_deref(), Some("def\ncustom: keep"));
        assert_eq!(issues[0].code, "unknown_rngagi_notes");
    }

    #[test]
    fn creates_a_staged_project_with_normalized_aggregates_and_metadata() {
        let (directory, path) = write_fixture(
            "form,gloss,root,example,translation,pos,domain\ne\u{301},first,r1;r2,sentence,translation,verb,Motion\ne\u{301},second,r1;r2,,,,Motion\n"
                .as_bytes(),
        );
        let mut request = request(path);
        request.project.parent_dir = directory.path().to_string_lossy().into_owned();
        request.project.name = "Imported".into();
        request.mappings.extend([
            CsvColumnMapping {
                column_index: 2,
                target: CsvMappingTarget::RootFallback,
            },
            CsvColumnMapping {
                column_index: 3,
                target: CsvMappingTarget::ExampleForm {
                    writing_system_id: "primary".into(),
                },
            },
            CsvColumnMapping {
                column_index: 4,
                target: CsvMappingTarget::ExampleTranslation,
            },
            CsvColumnMapping {
                column_index: 5,
                target: CsvMappingTarget::PartOfSpeech,
            },
            CsvColumnMapping {
                column_index: 6,
                target: CsvMappingTarget::SemanticDomain,
            },
        ]);
        let preview = preview(&request).expect("preview");
        assert_eq!(preview.blocking_error_count, 0);
        let (session, result) = create(CreateProjectFromCsvRequest {
            preview: request,
            preview_token: preview.preview_token,
        })
        .expect("import");
        assert_eq!(result.imported_entry_count, 1);
        assert_eq!(result.imported_sense_count, 2);
        assert_eq!(result.snapshot.part_of_speech_options, vec!["verb"]);
        assert_eq!(result.snapshot.semantic_domain_options, vec!["Motion"]);
        let entry = session
            .load_entry(&result.snapshot.entries[0].id)
            .expect("entry");
        assert_eq!(entry.forms[0].text, "é");
        assert_eq!(entry.senses.len(), 2);
        assert_eq!(entry.senses[0].examples.len(), 1);
        assert_eq!(
            entry
                .relations
                .iter()
                .map(|relation| relation.fallback_text.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("r1"), Some("r2")]
        );
        assert_eq!(
            result
                .snapshot
                .export_settings
                .corpus
                .part_of_speech_mappings
                .get("verb"),
            Some(&CorpusPartOfSpeech::Verb)
        );
        session.close().expect("close");
        assert!(directory.path().join("Imported.bkuw").is_dir());
        assert!(
            !fs::read_dir(directory.path())
                .expect("parent")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".bkuw-import-"))
        );
    }

    #[test]
    fn replacing_the_source_after_preview_leaves_no_project() {
        let (directory, path) = write_fixture(b"form,gloss\na,A\n");
        let mut request = request(path.clone());
        request.project.parent_dir = directory.path().to_string_lossy().into_owned();
        request.project.name = "Changed".into();
        let token = preview(&request).expect("preview").preview_token;
        fs::write(path, b"form,gloss\nb,B\n").expect("replace source");
        let error = match create(CreateProjectFromCsvRequest {
            preview: request,
            preview_token: token,
        }) {
            Ok(_) => panic!("changed source was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.code, "stale_preview");
        assert!(!directory.path().join("Changed.bkuw").exists());
    }
}
