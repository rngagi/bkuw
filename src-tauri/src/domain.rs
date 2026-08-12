use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub language_name: Option<String>,
    pub language_code: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshot {
    pub root_path: String,
    pub project: Project,
    pub writing_systems: Vec<WritingSystem>,
    pub part_of_speech_options: Vec<String>,
    pub semantic_domain_options: Vec<String>,
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
