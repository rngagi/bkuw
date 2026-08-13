use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub language_name: Option<String>,
    pub language_code: Option<String>,
    pub analysis_language: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorpusPartOfSpeech {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Pronoun,
    Particle,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CorpusExportSettings {
    pub part_of_speech_mappings: BTreeMap<String, CorpusPartOfSpeech>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SectionMode {
    Auto,
    FirstGrapheme,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReverseIndexMode {
    Gloss,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FontPreset {
    Auto,
    CharisSil,
    NotoSerif,
    NotoSerifCjkTc,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RelatedEntriesMode {
    #[default]
    None,
    Root,
    Base,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LatexExportSettings {
    pub title: String,
    pub author: String,
    pub headword_writing_system_id: String,
    pub pronunciation_writing_system_id: Option<String>,
    pub example_writing_system_id: String,
    pub collation_language_tag: Option<String>,
    pub section_mode: SectionMode,
    pub reverse_index: ReverseIndexMode,
    #[serde(default)]
    pub related_entries: RelatedEntriesMode,
    pub font_presets: BTreeMap<String, FontPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettingsV1 {
    pub version: u8,
    pub corpus: CorpusExportSettings,
    pub latex: LatexExportSettings,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExportKind {
    CorpusCsv,
    Latex,
    Pdf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExportIssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportIssue {
    pub severity: ExportIssueSeverity,
    pub code: String,
    pub entry_id: Option<String>,
    pub sense_id: Option<String>,
    pub field: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OmittedExportData {
    pub examples: usize,
    pub example_forms: usize,
    pub base_relations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreview {
    pub snapshot_token: String,
    pub row_count: usize,
    pub issues: Vec<ExportIssue>,
    pub omitted: OmittedExportData,
    pub required_font_packs: Vec<FontPackStatus>,
}

impl ExportPreview {
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == ExportIssueSeverity::Error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportProjectRequest {
    pub kind: ExportKind,
    pub destination: String,
    pub snapshot_token: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PdfStatus {
    NotRequested,
    Created,
    XeLatexMissing,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub csv_path: Option<String>,
    pub latex_directory: Option<String>,
    pub zip_path: Option<String>,
    pub pdf_path: Option<String>,
    pub pdf_status: PdfStatus,
    pub row_count: usize,
    pub issues: Vec<ExportIssue>,
    pub diagnostic_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TexEngineStatus {
    pub available: bool,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FontPackState {
    Missing,
    Installed,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FontPackStatus {
    pub id: String,
    pub version: String,
    pub state: FontPackState,
    pub mandatory: bool,
    pub installed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WritingSystem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub script_code: Option<String>,
    pub language_tag: Option<String>,
    pub display_role: Option<String>,
    pub sort_order: i64,
    pub font_family: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntryForm {
    pub id: String,
    pub writing_system_id: String,
    pub text: String,
    pub variant_label: Option<String>,
    pub dialect: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExampleForm {
    pub id: String,
    pub writing_system_id: String,
    pub text: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub id: String,
    pub translation: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i64,
    pub forms: Vec<ExampleForm>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Sense {
    pub id: String,
    pub gloss: Option<String>,
    pub definition: Option<String>,
    pub part_of_speech: Option<String>,
    pub semantic_domain: Option<String>,
    pub sort_order: i64,
    pub examples: Vec<Example>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntryRelation {
    pub id: String,
    pub target_entry_id: Option<String>,
    pub relation_type: String,
    pub fallback_text: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LexicalEntry {
    pub id: String,
    pub notes: Option<String>,
    pub section_override: Option<String>,
    pub revision: i64,
    pub created_at: String,
    pub updated_at: String,
    pub forms: Vec<EntryForm>,
    pub senses: Vec<Sense>,
    pub relations: Vec<EntryRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: String,
    pub primary_form: String,
    pub secondary_form: Option<String>,
    pub parts_of_speech: Vec<String>,
    pub revision: i64,
    pub section_label: Option<String>,
    pub manual_order_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntrySortMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EntrySortSettingsV1 {
    pub version: u8,
    pub mode: EntrySortMode,
    pub writing_system_id: String,
    pub alphabet: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ManualSortItem {
    Heading { id: String, label: String },
    Entry { entry_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManualSortLayoutV1 {
    pub version: u8,
    pub items: Vec<ManualSortItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub root_path: String,
    pub project: Project,
    pub writing_systems: Vec<WritingSystem>,
    pub part_of_speech_options: Vec<String>,
    pub semantic_domain_options: Vec<String>,
    pub export_settings: ExportSettingsV1,
    pub entry_sort_settings: EntrySortSettingsV1,
    pub manual_sort_layout: ManualSortLayoutV1,
    pub entries: Vec<EntrySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub parent_dir: String,
    pub name: String,
    pub language_name: Option<String>,
    pub language_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectSettingsRequest {
    pub name: String,
    pub language_name: Option<String>,
    pub language_code: Option<String>,
    pub analysis_language: Option<String>,
    pub description: Option<String>,
    pub writing_systems: Vec<WritingSystem>,
    pub part_of_speech_options: Vec<String>,
    pub semantic_domain_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveEntryRequest {
    pub entry: LexicalEntry,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteEntryRequest {
    pub id: String,
    pub expected_revision: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeletedEntry {
    pub id: String,
    pub deleted_at: String,
}
