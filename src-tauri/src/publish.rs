use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::Utc;
use pulldown_cmark::{Event, Options, Parser, Tag, html};
use reqwest::blocking::{Client, multipart};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tauri::ipc::Channel;
use unicode_normalization::UnicodeNormalization;

use crate::{
    domain::{
        CloudflareConnectionStatus, PublishDeploymentState, PublishIssue, PublishIssueSeverity,
        PublishPreview, PublishProgress, PublishResult, PublishSettingsV1, WritingSystem,
    },
    error::{AppError, AppResult},
    export::ExportSnapshot,
};

const API_ROOT: &str = "https://api.cloudflare.com/client/v4";
const STATIC_ASSET_LIMIT: usize = 25 * 1024 * 1024;
const WEBSITE_INDEX: &str = include_str!("../templates/publish-site/index.html");
const WEBSITE_STYLE: &str = include_str!("../templates/publish-site/styles.css");
const WEBSITE_APP: &str = include_str!("../templates/publish-site/app.js");
const WEBSITE_HEADERS: &str = include_str!("../templates/publish-site/_headers");

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PublishSnapshot {
    pub export: ExportSnapshot,
    pub audio: Vec<PublishAudio>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublishAudio {
    pub id: String,
    pub sense_id: Option<String>,
    pub example_id: Option<String>,
    pub relative_path: String,
    pub original_filename: String,
    pub duration_ms: i64,
    pub byte_size: i64,
    pub sha256: String,
    pub sort_order: i64,
}

#[derive(Default)]
pub struct PublishRuntime {
    session_tokens: Mutex<HashMap<String, String>>,
    persisted_accounts: Mutex<BTreeSet<String>>,
    active_account: Mutex<Option<String>>,
    cancel: AtomicBool,
}

impl PublishRuntime {
    pub fn remember_token(&self, account_id: &str, token: &str) -> bool {
        if let Ok(mut values) = self.session_tokens.lock() {
            values.insert(account_id.to_owned(), token.to_owned());
        }
        if let Ok(mut active) = self.active_account.lock() {
            *active = Some(account_id.to_owned());
        }
        let persisted = persist_token(account_id, token);
        if let Ok(mut accounts) = self.persisted_accounts.lock() {
            if persisted {
                accounts.insert(account_id.to_owned());
            } else {
                accounts.remove(account_id);
            }
        }
        persisted
    }

    pub fn token(&self, account_id: &str) -> AppResult<String> {
        if let Ok(values) = self.session_tokens.lock()
            && let Some(value) = values.get(account_id)
        {
            return Ok(value.clone());
        }
        let persisted = load_persisted_token(account_id);
        if persisted.is_some()
            && let Ok(mut accounts) = self.persisted_accounts.lock()
        {
            accounts.insert(account_id.to_owned());
        }
        persisted.ok_or_else(|| {
            AppError::new(
                "cloudflare_not_connected",
                "Connect this Cloudflare account before publishing.",
            )
        })
    }

    pub fn active_account(&self) -> Option<String> {
        self.active_account
            .lock()
            .ok()
            .and_then(|value| value.clone())
    }

    pub fn credential_persisted(&self, account_id: &str) -> bool {
        self.persisted_accounts
            .lock()
            .is_ok_and(|accounts| accounts.contains(account_id))
            || load_persisted_token(account_id).is_some()
    }

    pub fn disconnect(&self, account_id: &str) {
        if let Ok(mut values) = self.session_tokens.lock() {
            values.remove(account_id);
        }
        if let Ok(mut active) = self.active_account.lock()
            && active.as_deref() == Some(account_id)
        {
            *active = None;
        }
        if let Ok(mut accounts) = self.persisted_accounts.lock() {
            accounts.remove(account_id);
        }
        delete_persisted_token(account_id);
    }

    pub fn start(&self) {
        self.cancel.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    fn check_cancelled(&self) -> AppResult<()> {
        if self.cancel.load(Ordering::SeqCst) {
            Err(AppError::new(
                "publish_cancelled",
                "Website publishing was cancelled.",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(any(target_os = "macos", windows))]
fn credential_entry(account_id: &str) -> Option<keyring::Entry> {
    keyring::Entry::new("app.bkuw.desktop.cloudflare", account_id).ok()
}

#[cfg(any(target_os = "macos", windows))]
fn persist_token(account_id: &str, token: &str) -> bool {
    credential_entry(account_id)
        .and_then(|entry| entry.set_password(token).ok())
        .is_some()
}

#[cfg(not(any(target_os = "macos", windows)))]
fn persist_token(_account_id: &str, _token: &str) -> bool {
    false
}

#[cfg(any(target_os = "macos", windows))]
fn load_persisted_token(account_id: &str) -> Option<String> {
    credential_entry(account_id).and_then(|entry| entry.get_password().ok())
}

#[cfg(not(any(target_os = "macos", windows)))]
fn load_persisted_token(_account_id: &str) -> Option<String> {
    None
}

#[cfg(any(target_os = "macos", windows))]
fn delete_persisted_token(account_id: &str) {
    if let Some(entry) = credential_entry(account_id) {
        let _ = entry.delete_credential();
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
fn delete_persisted_token(_account_id: &str) {}

pub fn resource_stem(project_name: &str, project_id: &str) -> String {
    let mut stem = slug(project_name)
        .chars()
        .filter(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || *value == '-')
        .collect::<String>();
    if stem.is_empty() {
        stem = "dictionary".into();
    }
    if stem.len() > 42 {
        stem.truncate(42);
    }
    let short = project_id
        .chars()
        .filter(|value| value.is_ascii_hexdigit())
        .take(8)
        .collect::<String>();
    format!(
        "bkuw-{}-{}",
        stem.trim_matches('-'),
        short.to_ascii_lowercase()
    )
    .trim_matches('-')
    .to_owned()
}

pub fn media_bucket_name(worker_name: &str) -> String {
    format!("{worker_name}-media")
}

pub fn validate_settings(settings: &PublishSettingsV1, systems: &[WritingSystem]) -> AppResult<()> {
    if settings.version != 1 {
        return Err(AppError::new(
            "publish_settings_invalid",
            "The website settings version is unsupported.",
        ));
    }
    if settings.title.trim().is_empty() {
        return Err(AppError::new(
            "publish_title_required",
            "Enter a website title.",
        ));
    }
    if settings.locale != "zh-TW" && settings.locale != "en" {
        return Err(AppError::new(
            "publish_locale_invalid",
            "Choose a supported website language.",
        ));
    }
    validate_resource_name(&settings.worker_name, 58, "publish_worker_name_invalid")?;
    validate_resource_name(&settings.bucket_name, 64, "publish_bucket_name_invalid")?;
    if settings.bucket_name != media_bucket_name(&settings.worker_name) {
        return Err(AppError::new(
            "publish_bucket_name_invalid",
            "The R2 bucket name must match the Worker name followed by -media.",
        ));
    }
    let selected = settings.writing_system_ids.iter().collect::<BTreeSet<_>>();
    let known = systems
        .iter()
        .map(|value| &value.id)
        .collect::<BTreeSet<_>>();
    if !selected.is_subset(&known) {
        return Err(AppError::new(
            "publish_writing_system_invalid",
            "A selected writing system no longer exists.",
        ));
    }
    let primary = systems
        .iter()
        .find(|value| value.display_role.as_deref() == Some("primary"))
        .or_else(|| systems.first());
    if primary.is_none_or(|value| !selected.contains(&value.id)) {
        return Err(AppError::new(
            "publish_primary_required",
            "The primary writing system must be public.",
        ));
    }
    validate_markdown_links(settings.info_markdown.as_deref().unwrap_or(""))?;
    Ok(())
}

fn validate_resource_name(value: &str, max: usize, code: &'static str) -> AppResult<()> {
    let valid = (3..=max).contains(&value.len())
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .chars()
            .all(|item| item.is_ascii_lowercase() || item.is_ascii_digit() || item == '-');
    if valid {
        Ok(())
    } else {
        Err(AppError::new(
            code,
            "Use 3–63 lowercase letters, numbers, or dashes.",
        ))
    }
}

pub fn token_template_url(account_id: &str) -> AppResult<String> {
    validate_account_id(account_id)?;
    Ok(format!(
        "https://dash.cloudflare.com/profile/api-tokens?permissionGroupKeys=%5B%7B%22key%22%3A%22workers_scripts%22%2C%22type%22%3A%22edit%22%7D%2C%7B%22key%22%3A%22workers_r2%22%2C%22type%22%3A%22edit%22%7D%5D&accountId={account_id}&zoneId=all&name=bkuw%20Publish%20Token"
    ))
}

fn validate_account_id(value: &str) -> AppResult<()> {
    if value.len() == 32 && value.chars().all(|item| item.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(AppError::new(
            "cloudflare_account_invalid",
            "Enter the 32-character Cloudflare Account ID.",
        ))
    }
}

pub fn connect(
    runtime: &PublishRuntime,
    account_id: &str,
    token: &str,
    requested_subdomain: Option<&str>,
) -> AppResult<CloudflareConnectionStatus> {
    validate_account_id(account_id)?;
    if token.trim().len() < 20 || token.chars().any(char::is_whitespace) {
        return Err(AppError::new(
            "cloudflare_token_invalid",
            "Paste the complete Cloudflare API Token.",
        ));
    }
    let client = CloudflareClient::new(account_id, token)?;
    client.list_buckets()?;
    let mut subdomain = client.get_subdomain()?;
    if subdomain.is_none()
        && let Some(value) = requested_subdomain.filter(|value| !value.trim().is_empty())
    {
        validate_resource_name(value, 63, "cloudflare_subdomain_invalid")?;
        client.create_subdomain(value)?;
        subdomain = Some(value.to_owned());
    }
    if subdomain.is_none() {
        return Err(AppError::new(
            "cloudflare_subdomain_missing",
            "Enter an account subdomain to create the workers.dev address.",
        ));
    }
    let credential_persisted = runtime.remember_token(account_id, token);
    Ok(CloudflareConnectionStatus {
        connected: true,
        account_id: Some(account_id.to_owned()),
        workers_subdomain: subdomain,
        credential_persisted,
    })
}

pub fn update_workers_subdomain(
    runtime: &PublishRuntime,
    requested_subdomain: &str,
) -> AppResult<CloudflareConnectionStatus> {
    validate_resource_name(requested_subdomain, 63, "cloudflare_subdomain_invalid")?;
    let account_id = runtime.active_account().ok_or_else(|| {
        AppError::new(
            "cloudflare_not_connected",
            "Connect Cloudflare before changing the account subdomain.",
        )
    })?;
    let token = runtime.token(&account_id)?;
    let client = CloudflareClient::new(&account_id, &token)?;
    client.create_subdomain(requested_subdomain)?;
    Ok(connection_status(
        runtime,
        Some(&account_id),
        Some(requested_subdomain.to_owned()),
    ))
}

pub fn connection_status(
    runtime: &PublishRuntime,
    account_id: Option<&str>,
    workers_subdomain: Option<String>,
) -> CloudflareConnectionStatus {
    let account = account_id
        .map(str::to_owned)
        .or_else(|| runtime.active_account());
    let connected = account
        .as_deref()
        .is_some_and(|id| runtime.token(id).is_ok());
    let credential_persisted = account
        .as_deref()
        .is_some_and(|id| connected && runtime.credential_persisted(id));
    CloudflareConnectionStatus {
        connected,
        account_id: account,
        workers_subdomain,
        credential_persisted,
    }
}

#[derive(Debug, Clone)]
struct MediaAsset {
    key: String,
    source: PathBuf,
    content_type: &'static str,
    size: u64,
    sha256: String,
}

struct PreparedPublication {
    assets: BTreeMap<String, Vec<u8>>,
    media: Vec<MediaAsset>,
    issues: Vec<PublishIssue>,
    snapshot_token: String,
    corpus_sha256: String,
    entry_count: usize,
    sense_count: usize,
    example_count: usize,
    image_count: usize,
    audio_count: usize,
}

pub fn preview(
    snapshot: &PublishSnapshot,
    settings: &PublishSettingsV1,
    runtime: &PublishRuntime,
    account_id: Option<&str>,
) -> AppResult<PublishPreview> {
    let prepared = prepare(snapshot, settings)?;
    let mut remote = BTreeMap::new();
    let mut public_url = None;
    if let Some(account_id) = account_id {
        let token = runtime.token(account_id)?;
        let client = CloudflareClient::new(account_id, &token)?;
        if client.bucket_exists(&settings.bucket_name)? {
            remote = client.list_objects(&settings.bucket_name, "media/")?;
        }
        if let Some(subdomain) = client.get_subdomain()? {
            public_url = Some(format!(
                "https://{}.{}.workers.dev",
                settings.worker_name, subdomain
            ));
        }
    }
    let desired = prepared
        .media
        .iter()
        .map(|item| (item.key.clone(), item.size))
        .collect::<BTreeMap<_, _>>();
    let upload = desired
        .iter()
        .filter(|(key, size)| remote.get(*key) != Some(*size))
        .collect::<Vec<_>>();
    let delete_media_count = remote
        .keys()
        .filter(|key| !desired.contains_key(*key))
        .count();
    let upload_bytes = upload.iter().map(|(_, size)| **size).sum();
    Ok(PublishPreview {
        snapshot_token: prepared.snapshot_token,
        public_url,
        entry_count: prepared.entry_count,
        sense_count: prepared.sense_count,
        example_count: prepared.example_count,
        image_count: prepared.image_count,
        audio_count: prepared.audio_count,
        upload_media_count: upload.len(),
        unchanged_media_count: desired.len().saturating_sub(upload.len()),
        delete_media_count,
        upload_bytes,
        issues: prepared.issues,
    })
}

pub fn publish(
    snapshot: &PublishSnapshot,
    settings: &PublishSettingsV1,
    account_id: &str,
    expected_snapshot: &str,
    runtime: &PublishRuntime,
    progress: &Channel<PublishProgress>,
) -> AppResult<(PublishResult, PublishDeploymentState)> {
    runtime.start();
    emit(progress, "validating", 0, 1, 0, 0);
    let prepared = prepare(snapshot, settings)?;
    if prepared.snapshot_token != expected_snapshot {
        return Err(AppError::new(
            "publish_stale",
            "The project changed after the website check.",
        ));
    }
    if prepared
        .issues
        .iter()
        .any(|value| value.severity == PublishIssueSeverity::Error)
    {
        return Err(AppError::new(
            "publish_validation",
            "Fix the website check errors before publishing.",
        ));
    }
    runtime.check_cancelled()?;
    let token = runtime.token(account_id)?;
    let client = CloudflareClient::new(account_id, &token)?;
    client.list_buckets()?;
    let subdomain = client.get_subdomain()?.ok_or_else(|| {
        AppError::new(
            "cloudflare_subdomain_missing",
            "Create a workers.dev account subdomain before publishing.",
        )
    })?;
    emit(progress, "preparing", 0, 1, 0, 0);
    client.ensure_bucket(&settings.bucket_name, &snapshot.export.project.id)?;
    if !client.ensure_worker_available(&settings.worker_name, &snapshot.export.project.id)? {
        client.create_owned_worker(&settings.worker_name, &snapshot.export.project.id)?;
    }
    let remote = client.list_objects(&settings.bucket_name, "media/")?;
    let desired = prepared
        .media
        .iter()
        .map(|item| (item.key.clone(), item.size))
        .collect::<BTreeMap<_, _>>();
    let uploads = prepared
        .media
        .iter()
        .filter(|item| remote.get(&item.key) != Some(&item.size))
        .collect::<Vec<_>>();
    let stale = remote
        .keys()
        .filter(|key| !desired.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let total_bytes = uploads.iter().map(|item| item.size).sum();
    let mut uploaded_bytes = 0;
    emit(progress, "uploadingMedia", 0, uploads.len(), 0, total_bytes);
    for (index, asset) in uploads.iter().enumerate() {
        runtime.check_cancelled()?;
        client.upload_object(&settings.bucket_name, asset)?;
        uploaded_bytes += asset.size;
        emit(
            progress,
            "uploadingMedia",
            index + 1,
            uploads.len(),
            uploaded_bytes,
            total_bytes,
        );
    }
    runtime.check_cancelled()?;
    emit(progress, "uploadingAssets", 0, prepared.assets.len(), 0, 0);
    let completion = client.upload_assets(&settings.worker_name, &prepared.assets, progress)?;
    runtime.check_cancelled()?;
    emit(progress, "deploying", 0, 1, 0, 0);
    let worker_version_id = client.deploy_worker(
        &settings.worker_name,
        &settings.bucket_name,
        &snapshot.export.project.id,
        &completion,
    )?;
    client.enable_worker_subdomain(&settings.worker_name)?;
    let public_url = format!("https://{}.{}.workers.dev", settings.worker_name, subdomain);
    emit(progress, "verifying", 0, 1, 0, 0);
    client.health_check(
        &public_url,
        prepared.media.first().map(|value| value.key.as_str()),
        &prepared.corpus_sha256,
    )?;
    let mut cleanup_pending = false;
    let mut deleted = 0;
    emit(progress, "cleaning", 0, stale.len(), 0, 0);
    for (index, key) in stale.iter().enumerate() {
        if client.delete_object(&settings.bucket_name, key).is_ok() {
            deleted += 1;
        } else {
            cleanup_pending = true;
        }
        emit(progress, "cleaning", index + 1, stale.len(), 0, 0);
    }
    let now = Utc::now().to_rfc3339();
    let deployment = PublishDeploymentState {
        version: 1,
        account_id: account_id.to_owned(),
        worker_name: settings.worker_name.clone(),
        bucket_name: settings.bucket_name.clone(),
        workers_subdomain: subdomain,
        public_url: public_url.clone(),
        worker_version_id,
        corpus_sha256: prepared.corpus_sha256,
        last_published_at: now.clone(),
        cleanup_pending,
    };
    emit(progress, "complete", 1, 1, uploaded_bytes, total_bytes);
    Ok((
        PublishResult {
            public_url,
            uploaded_media_count: uploads.len(),
            unchanged_media_count: desired.len().saturating_sub(uploads.len()),
            deleted_media_count: deleted,
            cleanup_pending,
            last_published_at: now,
        },
        deployment,
    ))
}

pub fn retry_cleanup(
    runtime: &PublishRuntime,
    deployment: &PublishDeploymentState,
    desired_keys: &BTreeSet<String>,
) -> AppResult<usize> {
    let token = runtime.token(&deployment.account_id)?;
    let client = CloudflareClient::new(&deployment.account_id, &token)?;
    let remote = client.list_objects(&deployment.bucket_name, "media/")?;
    let mut deleted = 0;
    for key in remote.keys().filter(|key| !desired_keys.contains(*key)) {
        client.delete_object(&deployment.bucket_name, key)?;
        deleted += 1;
    }
    Ok(deleted)
}

pub fn retry_cleanup_for_snapshot(
    runtime: &PublishRuntime,
    deployment: &PublishDeploymentState,
    snapshot: &PublishSnapshot,
    settings: &PublishSettingsV1,
) -> AppResult<usize> {
    let desired = prepare(snapshot, settings)?
        .media
        .into_iter()
        .map(|value| value.key)
        .collect::<BTreeSet<_>>();
    retry_cleanup(runtime, deployment, &desired)
}

fn emit(
    channel: &Channel<PublishProgress>,
    phase: &str,
    completed_items: usize,
    total_items: usize,
    uploaded_bytes: u64,
    total_bytes: u64,
) {
    let _ = channel.send(PublishProgress {
        phase: phase.to_owned(),
        completed_items,
        total_items,
        uploaded_bytes,
        total_bytes,
    });
}

fn prepare(
    snapshot: &PublishSnapshot,
    settings: &PublishSettingsV1,
) -> AppResult<PreparedPublication> {
    validate_settings(settings, &snapshot.export.writing_systems)?;
    let selected = settings
        .writing_system_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let primary = snapshot
        .export
        .writing_systems
        .iter()
        .find(|value| value.display_role.as_deref() == Some("primary"))
        .or_else(|| snapshot.export.writing_systems.first())
        .ok_or_else(|| {
            AppError::new(
                "publish_primary_required",
                "Create a writing system before publishing.",
            )
        })?;
    let mut issues = Vec::new();
    let mut slugs = HashMap::new();
    let mut used = BTreeSet::new();
    for entry in &snapshot.export.entries {
        let headword = entry
            .forms
            .iter()
            .find(|value| value.writing_system_id == primary.id)
            .map(|value| value.text.trim())
            .unwrap_or("");
        let mut value = slug(headword);
        if value.is_empty() {
            value = format!("entry-{}", short_id(&entry.id));
        }
        if !used.insert(value.clone()) {
            value = format!("{value}-{}", short_id(&entry.id));
            used.insert(value.clone());
        }
        slugs.insert(entry.id.clone(), value);
        if headword.is_empty() {
            issues.push(PublishIssue {
                severity: PublishIssueSeverity::Error,
                code: "publish.primary_form_missing".into(),
                entry_id: Some(entry.id.clone()),
                details: None,
            });
        }
    }
    let images_by_sense = snapshot.export.sense_images.iter().fold(
        HashMap::<&str, Vec<_>>::new(),
        |mut map, value| {
            map.entry(&value.sense_id).or_default().push(value);
            map
        },
    );
    let audio_by_sense = snapshot
        .audio
        .iter()
        .filter_map(|value| value.sense_id.as_deref().map(|id| (id, value)))
        .fold(HashMap::<&str, Vec<_>>::new(), |mut map, (id, value)| {
            map.entry(id).or_default().push(value);
            map
        });
    let audio_by_example = snapshot
        .audio
        .iter()
        .filter_map(|value| value.example_id.as_deref().map(|id| (id, value)))
        .fold(HashMap::<&str, Vec<_>>::new(), |mut map, (id, value)| {
            map.entry(id).or_default().push(value);
            map
        });
    let headword_by_id = snapshot
        .export
        .entries
        .iter()
        .map(|entry| {
            let value = entry
                .forms
                .iter()
                .find(|form| form.writing_system_id == primary.id)
                .map(|form| form.text.clone())
                .unwrap_or_default();
            (entry.id.clone(), value)
        })
        .collect::<HashMap<_, _>>();
    let mut media_by_key = BTreeMap::<String, MediaAsset>::new();
    let mut senses_count = 0;
    let mut examples_count = 0;
    let mut image_count = 0;
    let mut audio_count = 0;
    let entries = snapshot.export.entries.iter().map(|entry| {
        let forms = entry.forms.iter().filter(|form| selected.contains(&form.writing_system_id)).map(|form| json!({
            "id": form.id, "writingSystemId": form.writing_system_id, "text": form.text,
            "variantLabel": form.variant_label, "dialect": form.dialect, "status": form.status,
            "sortOrder": form.sort_order
        })).collect::<Vec<_>>();
        let primary_form = entry.forms.iter().find(|form| form.writing_system_id == primary.id).map(|form| form.text.clone()).unwrap_or_default();
        let senses = entry.senses.iter().map(|sense| {
            senses_count += 1;
            let images = images_by_sense.get(sense.id.as_str()).into_iter().flatten().filter_map(|image| {
                match local_media(&snapshot.export.root_path, &image.relative_path, &image.sha256, "images", "image/png") {
                    Ok(media) => {
                        image_count += 1;
                        let key = media.key.clone(); media_by_key.entry(key.clone()).or_insert(media);
                        Some(json!({"id":image.id,"path":key.trim_start_matches("media/"),"originalFilename":image.original_filename,"width":image.width,"height":image.height,"byteSize":image.byte_size}))
                    }
                    Err(error) => { issues.push(PublishIssue { severity: PublishIssueSeverity::Error, code: error.code.into(), entry_id: Some(entry.id.clone()), details: error.details }); None }
                }
            }).collect::<Vec<_>>();
            let sense_audio = audio_by_sense.get(sense.id.as_str()).into_iter().flatten().filter_map(|item| {
                match local_media(&snapshot.export.root_path, &item.relative_path, &item.sha256, "audio", "audio/webm") {
                    Ok(media) => { audio_count += 1; let key=media.key.clone(); media_by_key.entry(key.clone()).or_insert(media); Some(audio_json(item, &key)) }
                    Err(error) => { issues.push(PublishIssue { severity: PublishIssueSeverity::Error, code:error.code.into(), entry_id:Some(entry.id.clone()), details:error.details }); None }
                }
            }).collect::<Vec<_>>();
            let examples = sense.examples.iter().map(|example| {
                examples_count += 1;
                let example_audio = audio_by_example.get(example.id.as_str()).into_iter().flatten().filter_map(|item| {
                    match local_media(&snapshot.export.root_path, &item.relative_path, &item.sha256, "audio", "audio/webm") {
                        Ok(media) => { audio_count += 1; let key=media.key.clone(); media_by_key.entry(key.clone()).or_insert(media); Some(audio_json(item, &key)) }
                        Err(error) => { issues.push(PublishIssue { severity: PublishIssueSeverity::Error, code:error.code.into(), entry_id:Some(entry.id.clone()), details:error.details }); None }
                    }
                }).collect::<Vec<_>>();
                json!({
                    "id":example.id,
                    "translation":example.translation,
                    "notes":if settings.include_example_notes { example.notes.clone() } else { None },
                    "sortOrder":example.sort_order,
                    "forms":example.forms.iter().filter(|form| selected.contains(&form.writing_system_id)).map(|form|json!({"id":form.id,"writingSystemId":form.writing_system_id,"text":form.text,"sortOrder":form.sort_order})).collect::<Vec<_>>(),
                    "audio":example_audio
                })
            }).collect::<Vec<_>>();
            json!({"id":sense.id,"gloss":sense.gloss,"definition":sense.definition,"partOfSpeech":sense.part_of_speech,"semanticDomain":sense.semantic_domain,"sortOrder":sense.sort_order,"images":images,"audio":sense_audio,"examples":examples})
        }).collect::<Vec<_>>();
        let relations = if settings.include_relations { entry.relations.iter().map(|relation| json!({
            "id":relation.id,"type":relation.relation_type,"targetEntryId":relation.target_entry_id,
            "targetSlug":relation.target_entry_id.as_ref().and_then(|id| slugs.get(id)),
            "targetHeadword":relation.target_entry_id.as_ref().and_then(|id| headword_by_id.get(id)),
            "fallbackText":relation.fallback_text,"sortOrder":relation.sort_order
        })).collect::<Vec<_>>() } else { Vec::new() };
        json!({
            "id":entry.id,
            "slug":slugs.get(&entry.id),
            "sectionLabel":snapshot.export.sections.get(&entry.id).cloned().flatten(),
            "primaryForm":primary_form,
            "notes":if settings.include_entry_notes {entry.notes.clone()} else {None},
            "forms":forms,
            "senses":senses,
            "relations":relations
        })
    }).collect::<Vec<_>>();
    let info_html = settings
        .info_markdown
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(markdown_to_html)
        .transpose()?;
    let corpus = json!({
        "schemaVersion":1,
        "site":{"title":settings.title.trim(),"description":settings.description,"defaultLocale":settings.locale,"defaultTheme":"system","defaultEntryId":snapshot.export.entries.first().map(|value|&value.id),"mediaBaseUrl":"./media/","info":info_html.map(|html|json!({"html":html})),"publicFields":{"entryNotes":settings.include_entry_notes,"exampleNotes":settings.include_example_notes,"relations":settings.include_relations}},
        "project":{"id":snapshot.export.project.id,"name":snapshot.export.project.name,"languageName":snapshot.export.project.language_name,"languageCode":snapshot.export.project.language_code,"analysisLanguage":snapshot.export.project.analysis_language},
        "writingSystems":snapshot.export.writing_systems.iter().filter(|value|selected.contains(&value.id)).map(|value|json!({
            "id":value.id,"name":value.name,"type":value.kind,"scriptCode":value.script_code,
            "languageTag":value.language_tag,"displayRole":value.display_role,
            "sortOrder":value.sort_order,"fontFamily":value.font_family
        })).collect::<Vec<_>>(),
        "stats":{"entries":entries.len(),"senses":senses_count,"examples":examples_count,"images":image_count,"audio":audio_count},
        "entries":entries
    });
    let corpus_bytes = serde_json::to_vec_pretty(&corpus).map_err(json_error)?;
    if corpus_bytes.len() > STATIC_ASSET_LIMIT {
        issues.push(PublishIssue {
            severity: PublishIssueSeverity::Error,
            code: "publish.corpus_too_large".into(),
            entry_id: None,
            details: Some(corpus_bytes.len().to_string()),
        });
    }
    let corpus_sha256 = hex_sha256(&corpus_bytes);
    let mut assets = BTreeMap::from([
        ("/index.html".into(), WEBSITE_INDEX.as_bytes().to_vec()),
        ("/styles.css".into(), WEBSITE_STYLE.as_bytes().to_vec()),
        ("/app.js".into(), WEBSITE_APP.as_bytes().to_vec()),
        ("/_headers".into(), WEBSITE_HEADERS.as_bytes().to_vec()),
        ("/data/corpus.json".into(), corpus_bytes),
    ]);
    assets.insert("/favicon.svg".into(), favicon().into_bytes());
    let token_input = serde_json::to_vec(&(settings, &corpus_sha256)).map_err(json_error)?;
    Ok(PreparedPublication {
        assets,
        media: media_by_key.into_values().collect(),
        issues,
        snapshot_token: hex_sha256(&token_input),
        corpus_sha256,
        entry_count: snapshot.export.entries.len(),
        sense_count: senses_count,
        example_count: examples_count,
        image_count,
        audio_count,
    })
}

fn audio_json(item: &PublishAudio, key: &str) -> Value {
    json!({"id":item.id,"path":key.trim_start_matches("media/"),"originalFilename":item.original_filename,"durationMs":item.duration_ms,"byteSize":item.byte_size})
}

fn local_media(
    root: &Path,
    relative: &str,
    expected_sha: &str,
    kind: &str,
    content_type: &'static str,
) -> AppResult<MediaAsset> {
    let path = root.join(relative);
    let canonical_root = root.canonicalize()?;
    let canonical = path.canonicalize().map_err(|error| {
        AppError::with_details(
            "publish_media_missing",
            "A website media file is missing.",
            error.to_string(),
        )
    })?;
    if !canonical.starts_with(&canonical_root) {
        return Err(AppError::new(
            "publish_media_invalid",
            "A website media path leaves the project.",
        ));
    }
    let bytes = fs::read(&canonical)?;
    let sha256 = hex_sha256(&bytes);
    if sha256 != expected_sha.to_ascii_lowercase() {
        return Err(AppError::with_details(
            "publish_media_hash",
            "A website media file failed its integrity check.",
            relative,
        ));
    }
    let extension = if kind == "images" { "png" } else { "webm" };
    Ok(MediaAsset {
        key: format!("media/{kind}/{sha256}.{extension}"),
        source: canonical,
        content_type,
        size: bytes.len() as u64,
        sha256,
    })
}

fn markdown_to_html(markdown: &str) -> AppResult<String> {
    validate_markdown_links(markdown)?;
    let escaped = markdown.replace('<', "&lt;").replace('>', "&gt;");
    let mut output = String::new();
    html::push_html(
        &mut output,
        Parser::new_ext(&escaped, Options::ENABLE_STRIKETHROUGH),
    );
    Ok(output)
}

fn validate_markdown_links(markdown: &str) -> AppResult<()> {
    for event in Parser::new_ext(markdown, Options::empty()) {
        if let Event::Start(Tag::Link { dest_url, .. }) = event {
            let target = dest_url.trim();
            if !(target.starts_with("https://")
                || target.starts_with("http://")
                || target.starts_with("mailto:")
                || target.starts_with('#')
                || target.starts_with("./"))
            {
                return Err(AppError::with_details(
                    "publish_markdown_link",
                    "The website information contains an unsafe link.",
                    target,
                ));
            }
        }
    }
    Ok(())
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    let mut dash = false;
    for value in value
        .nfkd()
        .filter(|value| !unicode_normalization::char::is_combining_mark(*value))
    {
        for lower in value.to_lowercase() {
            if lower.is_alphanumeric() {
                output.push(lower);
                dash = false;
            } else if !output.is_empty() && !dash {
                output.push('-');
                dash = true;
            }
        }
    }
    output.trim_matches('-').to_owned()
}

fn short_id(value: &str) -> String {
    value
        .chars()
        .filter(|item| item.is_alphanumeric())
        .take(8)
        .collect::<String>()
        .to_lowercase()
}
fn hex_sha256(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}
fn json_error(error: serde_json::Error) -> AppError {
    AppError::with_details(
        "publish_json",
        "The website data could not be generated.",
        error.to_string(),
    )
}
fn favicon() -> String {
    "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><rect width=\"64\" height=\"64\" rx=\"12\" fill=\"#7f1d2d\"/><path d=\"M18 14v36h8v-3c3 3 7 4 11 4 9 0 15-7 15-17s-6-17-15-17c-4 0-8 2-11 5v-8zm17 11c5 0 8 4 8 9s-3 9-8 9-9-4-9-9 4-9 9-9z\" fill=\"#fcfcfa\"/></svg>".into()
}

#[derive(Deserialize)]
struct Envelope<T> {
    success: bool,
    result: Option<T>,
    #[serde(default)]
    errors: Option<Vec<ApiMessage>>,
}
#[derive(Deserialize)]
struct ApiMessage {
    code: Option<i64>,
    message: String,
}
#[derive(Deserialize)]
struct Subdomain {
    subdomain: String,
}
#[derive(Deserialize)]
struct BucketList {
    #[serde(default)]
    buckets: Vec<Bucket>,
}
#[derive(Deserialize)]
struct Bucket {
    name: Option<String>,
}
#[derive(Deserialize)]
struct UploadSession {
    jwt: Option<String>,
    #[serde(default)]
    buckets: Vec<Vec<String>>,
}
#[derive(Deserialize)]
struct ObjectInfo {
    key: Option<String>,
    size: Option<u64>,
}
#[derive(Deserialize, Default)]
struct ResultInfo {
    cursor: Option<String>,
    #[serde(default)]
    is_truncated: bool,
}

struct CloudflareClient<'a> {
    account_id: &'a str,
    token: &'a str,
    api_root: String,
    client: Client,
}

impl<'a> CloudflareClient<'a> {
    fn new(account_id: &'a str, token: &'a str) -> AppResult<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(http_error)?;
        Ok(Self {
            account_id,
            token,
            api_root: API_ROOT.into(),
            client,
        })
    }

    #[cfg(test)]
    fn new_with_root(account_id: &'a str, token: &'a str, api_root: String) -> AppResult<Self> {
        let mut client = Self::new(account_id, token)?;
        client.api_root = api_root;
        Ok(client)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/accounts/{}{path}", self.api_root, self.account_id)
    }
    fn auth(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        request.bearer_auth(self.token)
    }

    fn send_with_retry<F>(&self, mut build: F) -> AppResult<reqwest::blocking::Response>
    where
        F: FnMut() -> reqwest::blocking::RequestBuilder,
    {
        let mut last_error = None;
        for attempt in 0..3 {
            match build().send() {
                Ok(response) => {
                    let retryable =
                        response.status().as_u16() == 429 || response.status().is_server_error();
                    if !retryable || attempt == 2 {
                        return Ok(response);
                    }
                    let retry_after = response
                        .headers()
                        .get(reqwest::header::RETRY_AFTER)
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(1_u64 << attempt)
                        .min(8);
                    thread::sleep(Duration::from_secs(retry_after));
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                    if attempt < 2 {
                        thread::sleep(Duration::from_secs(1_u64 << attempt));
                    }
                }
            }
        }
        Err(AppError::with_details(
            "cloudflare_network",
            "Cloudflare could not be reached after several attempts.",
            last_error.unwrap_or_default(),
        ))
    }

    fn get_subdomain(&self) -> AppResult<Option<String>> {
        let response =
            self.send_with_retry(|| self.auth(self.client.get(self.url("/workers/subdomain"))))?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        let envelope: Envelope<Subdomain> = decode(response)?;
        Ok(envelope.result.map(|value| value.subdomain))
    }

    fn create_subdomain(&self, value: &str) -> AppResult<()> {
        let response = self.send_with_retry(|| {
            self.auth(self.client.put(self.url("/workers/subdomain")))
                .json(&json!({"subdomain":value}))
        })?;
        let _: Envelope<Subdomain> = decode(response)?;
        Ok(())
    }

    fn list_buckets(&self) -> AppResult<Vec<String>> {
        let response = self.send_with_retry(|| {
            self.auth(self.client.get(self.url("/r2/buckets?per_page=1000")))
        })?;
        let envelope: Envelope<BucketList> = decode(response)?;
        Ok(envelope
            .result
            .unwrap_or(BucketList {
                buckets: Vec::new(),
            })
            .buckets
            .into_iter()
            .filter_map(|value| value.name)
            .collect())
    }

    fn bucket_exists(&self, name: &str) -> AppResult<bool> {
        Ok(self.list_buckets()?.iter().any(|value| value == name))
    }

    fn ensure_bucket(&self, name: &str, project_id: &str) -> AppResult<()> {
        let created = !self.bucket_exists(name)?;
        if created {
            let response = self
                .auth(self.client.post(self.url("/r2/buckets")))
                .json(&json!({"name":name,"storageClass":"Standard"}))
                .send()
                .map_err(http_error)?;
            let _: Envelope<Value> = decode(response)?;
        }
        let marker = format!(
            "{{\"schemaVersion\":1,\"projectId\":{}}}",
            serde_json::to_string(project_id).map_err(json_error)?
        );
        let marker_url = self.object_url(name, ".bkuw/owner.json");
        let response = self.send_with_retry(|| self.auth(self.client.get(&marker_url)))?;
        if response.status().as_u16() == 404 {
            if !created {
                return Err(AppError::new(
                    "publish_bucket_owned",
                    "This existing R2 bucket is not managed by this bkuw project.",
                ));
            }
            self.upload_bytes(
                name,
                ".bkuw/owner.json",
                marker.as_bytes(),
                "application/json",
            )?;
        } else {
            let body = response
                .error_for_status()
                .map_err(http_error)?
                .text()
                .map_err(http_error)?;
            let value: Value = serde_json::from_str(&body).map_err(json_error)?;
            if value.get("projectId").and_then(Value::as_str) != Some(project_id) {
                return Err(AppError::new(
                    "publish_bucket_owned",
                    "This R2 bucket belongs to another project.",
                ));
            }
        }
        Ok(())
    }

    fn ensure_worker_available(&self, name: &str, project_id: &str) -> AppResult<bool> {
        let settings_url = self.url(&format!("/workers/scripts/{name}/settings"));
        let response = self.send_with_retry(|| self.auth(self.client.get(settings_url.clone())))?;
        if response.status().is_success() {
            let envelope: Envelope<Value> = decode(response)?;
            let expected = format!("bkuw:{project_id}");
            let owner = envelope
                .result
                .as_ref()
                .and_then(|value| value.get("annotations"))
                .and_then(|value| value.get("workers/tag"))
                .and_then(Value::as_str);
            if owner != Some(expected.as_str()) {
                return Err(AppError::new(
                    "publish_worker_owned",
                    "This existing Worker is not managed by this bkuw project.",
                ));
            }
            return Ok(true);
        }
        if response.status().as_u16() != 404 && !response.status().is_success() {
            let _: Envelope<Value> = decode(response)?;
        }
        Ok(false)
    }

    fn create_owned_worker(&self, name: &str, project_id: &str) -> AppResult<()> {
        let metadata = json!({
            "main_module":"main.js",
            "compatibility_date":"2026-09-20",
            "annotations":{
                "workers/message":"Reserved by bkuw",
                "workers/tag":format!("bkuw:{project_id}")
            }
        });
        let script = r#"export default { async fetch() { return new Response("Publishing in progress", { status: 503 }); } };"#;
        let form = multipart::Form::new()
            .part(
                "metadata",
                multipart::Part::text(metadata.to_string())
                    .mime_str("application/json")
                    .map_err(http_error)?,
            )
            .part(
                "main.js",
                multipart::Part::text(script)
                    .file_name("main.js")
                    .mime_str("application/javascript+module")
                    .map_err(http_error)?,
            );
        let response = self
            .auth(
                self.client
                    .put(self.url(&format!("/workers/scripts/{name}"))),
            )
            .multipart(form)
            .send();
        match response {
            Ok(value) => {
                let _: Envelope<Value> = decode(value)?;
                Ok(())
            }
            Err(error) => {
                thread::sleep(Duration::from_secs(2));
                if self.ensure_worker_available(name, project_id)? {
                    Ok(())
                } else {
                    Err(http_error(error))
                }
            }
        }
    }

    fn object_url(&self, bucket: &str, key: &str) -> String {
        let encoded = key
            .split('/')
            .map(percent_encode)
            .collect::<Vec<_>>()
            .join("/");
        self.url(&format!("/r2/buckets/{bucket}/objects/{encoded}"))
    }

    fn list_objects(&self, bucket: &str, prefix: &str) -> AppResult<BTreeMap<String, u64>> {
        let mut output = BTreeMap::new();
        let mut cursor: Option<String> = None;
        loop {
            let mut url = self.url(&format!(
                "/r2/buckets/{bucket}/objects?per_page=1000&prefix={}",
                percent_encode(prefix)
            ));
            if let Some(value) = &cursor {
                url.push_str("&cursor=");
                url.push_str(&percent_encode(value));
            }
            let response = self.send_with_retry(|| self.auth(self.client.get(url.clone())))?;
            if response.status().as_u16() == 404 {
                return Ok(output);
            }
            let value: Value = decode_value(response)?;
            let envelope: Envelope<Vec<ObjectInfo>> =
                serde_json::from_value(value.clone()).map_err(json_error)?;
            for item in envelope.result.unwrap_or_default() {
                if let (Some(key), Some(size)) = (item.key, item.size) {
                    output.insert(key, size);
                }
            }
            let info: ResultInfo =
                serde_json::from_value(value.get("result_info").cloned().unwrap_or_default())
                    .unwrap_or_default();
            if !info.is_truncated {
                break;
            }
            cursor = info.cursor;
            if cursor.is_none() {
                break;
            }
        }
        Ok(output)
    }

    fn upload_object(&self, bucket: &str, asset: &MediaAsset) -> AppResult<()> {
        let bytes = fs::read(&asset.source)?;
        if hex_sha256(&bytes) != asset.sha256 {
            return Err(AppError::new(
                "publish_media_hash",
                "A media file changed during publishing.",
            ));
        }
        self.upload_bytes(bucket, &asset.key, &bytes, asset.content_type)
    }

    fn upload_bytes(
        &self,
        bucket: &str,
        key: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> AppResult<()> {
        let object_url = self.object_url(bucket, key);
        let response = self.send_with_retry(|| {
            self.auth(self.client.put(object_url.clone()))
                .header(reqwest::header::CONTENT_TYPE, content_type)
                .body(bytes.to_vec())
        })?;
        let _: Envelope<Value> = decode(response)?;
        Ok(())
    }

    fn delete_object(&self, bucket: &str, key: &str) -> AppResult<()> {
        let object_url = self.object_url(bucket, key);
        let response =
            self.send_with_retry(|| self.auth(self.client.delete(object_url.clone())))?;
        let _: Envelope<Value> = decode(response)?;
        Ok(())
    }

    fn upload_assets(
        &self,
        worker: &str,
        assets: &BTreeMap<String, Vec<u8>>,
        progress: &Channel<PublishProgress>,
    ) -> AppResult<String> {
        let manifest = assets.iter().map(|(path, bytes)| {
            let extension = Path::new(path).extension().and_then(|value| value.to_str()).unwrap_or("");
            let mut input = BASE64.encode(bytes); input.push_str(extension);
            (path.clone(), json!({"hash":hex_sha256(input.as_bytes())[..32].to_owned(),"size":bytes.len()}))
        }).collect::<serde_json::Map<_, _>>();
        let session_url = self.url(&format!("/workers/scripts/{worker}/assets-upload-session"));
        let response = self.send_with_retry(|| {
            self.auth(self.client.post(session_url.clone()))
                .json(&json!({"manifest":manifest.clone()}))
        })?;
        let session: UploadSession =
            decode::<UploadSession>(response)?.result.ok_or_else(|| {
                AppError::new(
                    "cloudflare_response",
                    "Cloudflare did not return an asset upload session.",
                )
            })?;
        let lookup = assets
            .iter()
            .map(|(path, bytes)| {
                let extension = Path::new(path)
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                let mut input = BASE64.encode(bytes);
                input.push_str(extension);
                (hex_sha256(input.as_bytes())[..32].to_owned(), bytes)
            })
            .collect::<HashMap<_, _>>();
        let upload_jwt = session.jwt.ok_or_else(|| {
            AppError::new(
                "cloudflare_response",
                "Cloudflare did not return an asset upload token.",
            )
        })?;
        let mut completion = if session.buckets.is_empty() {
            Some(upload_jwt.clone())
        } else {
            None
        };
        for (index, bucket) in session.buckets.iter().enumerate() {
            let mut upload_parts = Vec::new();
            for hash in bucket {
                let bytes = lookup.get(hash).ok_or_else(|| {
                    AppError::new(
                        "publish_assets",
                        "Cloudflare requested an unknown website asset.",
                    )
                })?;
                let content_type = assets
                    .iter()
                    .find_map(|(path, value)| {
                        let extension = Path::new(path)
                            .extension()
                            .and_then(|item| item.to_str())
                            .unwrap_or("");
                        let mut input = BASE64.encode(value);
                        input.push_str(extension);
                        (hex_sha256(input.as_bytes())[..32] == *hash.as_str())
                            .then(|| asset_content_type(path))
                    })
                    .unwrap_or("application/octet-stream");
                upload_parts.push((hash.clone(), BASE64.encode(bytes), content_type));
            }
            let upload_url = format!(
                "{}/accounts/{}/workers/assets/upload?base64=true",
                self.api_root, self.account_id
            );
            let response = self.send_with_retry(|| {
                let form = upload_parts.iter().fold(
                    multipart::Form::new(),
                    |form, (hash, encoded, content_type)| {
                        let part = multipart::Part::text(encoded.clone())
                            .mime_str(content_type)
                            .expect("validated asset MIME type");
                        form.part(hash.clone(), part)
                    },
                );
                self.client
                    .post(upload_url.clone())
                    .bearer_auth(&upload_jwt)
                    .multipart(form)
            })?;
            let result: UploadSession =
                decode::<UploadSession>(response)?.result.ok_or_else(|| {
                    AppError::new(
                        "cloudflare_response",
                        "Cloudflare did not finish the asset upload.",
                    )
                })?;
            if let Some(jwt) = result.jwt {
                completion = Some(jwt);
            }
            emit(
                progress,
                "uploadingAssets",
                index + 1,
                session.buckets.len(),
                0,
                0,
            );
        }
        completion.ok_or_else(|| {
            AppError::new(
                "cloudflare_response",
                "Cloudflare did not return the completed asset upload token.",
            )
        })
    }

    fn deploy_worker(
        &self,
        worker: &str,
        bucket: &str,
        project_id: &str,
        completion: &str,
    ) -> AppResult<Option<String>> {
        let metadata = json!({
            "main_module":"main.js",
            "compatibility_date":"2026-09-20",
            "assets":{"jwt":completion,"config":{"run_worker_first":["/media/*"]}},
            "bindings":[{"type":"assets","name":"ASSETS"},{"type":"r2_bucket","name":"MEDIA","bucket_name":bucket}],
            "annotations":{"workers/message":"Published by bkuw","workers/tag":format!("bkuw:{project_id}")}
        });
        let script = worker_script(project_id);
        let form = multipart::Form::new()
            .part(
                "metadata",
                multipart::Part::text(metadata.to_string())
                    .mime_str("application/json")
                    .map_err(http_error)?,
            )
            .part(
                "main.js",
                multipart::Part::text(script)
                    .file_name("main.js")
                    .mime_str("application/javascript+module")
                    .map_err(http_error)?,
            );
        let response = self
            .auth(
                self.client
                    .put(self.url(&format!("/workers/scripts/{worker}"))),
            )
            .multipart(form)
            .send();
        let response = match response {
            Ok(value) => value,
            Err(error) => {
                thread::sleep(Duration::from_secs(2));
                if self.ensure_worker_available(worker, project_id)? {
                    return Ok(None);
                }
                return Err(http_error(error));
            }
        };
        let envelope: Envelope<Value> = decode(response)?;
        Ok(envelope
            .result
            .as_ref()
            .and_then(|value| value.get("etag").or_else(|| value.get("id")))
            .and_then(Value::as_str)
            .map(str::to_owned))
    }

    fn enable_worker_subdomain(&self, worker: &str) -> AppResult<()> {
        let route_url = self.url(&format!("/workers/scripts/{worker}/subdomain"));
        let response = self.send_with_retry(|| {
            self.auth(self.client.post(route_url.clone()))
                .json(&json!({"enabled":true,"previews_enabled":false}))
        })?;
        let _: Envelope<Value> = decode(response)?;
        Ok(())
    }

    fn health_check(
        &self,
        public_url: &str,
        media: Option<&str>,
        corpus_sha256: &str,
    ) -> AppResult<()> {
        for attempt in 0..8 {
            let cache = format!("?bkuw-check={}", Utc::now().timestamp_millis());
            let home = self.client.get(format!("{public_url}/{cache}")).send();
            let corpus_matches = self
                .client
                .get(format!("{public_url}/data/corpus.json{cache}"))
                .send()
                .ok()
                .filter(|value| value.status().is_success())
                .and_then(|value| value.bytes().ok())
                .is_some_and(|value| hex_sha256(&value) == corpus_sha256);
            let media_ok = media
                .map(|key| {
                    self.client
                        .head(format!("{public_url}/{key}{cache}"))
                        .send()
                        .is_ok_and(|value| value.status().is_success())
                })
                .unwrap_or(true);
            if home.is_ok_and(|value| value.status().is_success()) && corpus_matches && media_ok {
                return Ok(());
            }
            if attempt < 7 {
                thread::sleep(Duration::from_secs(8));
            }
        }
        Err(AppError::new(
            "publish_health_check",
            "The new website did not become reachable within 60 seconds.",
        ))
    }
}

fn asset_content_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

fn worker_script(project_id: &str) -> String {
    format!(
        r#"const PROJECT_ID = {project_id:?};
export default {{
  async fetch(request, env) {{
    const url = new URL(request.url);
    if (!url.pathname.startsWith('/media/')) return env.ASSETS.fetch(request);
    if (request.method !== 'GET' && request.method !== 'HEAD') return new Response('Method Not Allowed', {{status:405,headers:{{Allow:'GET, HEAD'}}}});
    const key = decodeURIComponent(url.pathname.slice(1));
    if (!key.startsWith('media/') || key.includes('..')) return new Response('Not Found', {{status:404}});
    const object = request.method === 'HEAD' ? await env.MEDIA.head(key) : await env.MEDIA.get(key, {{onlyIf:request.headers,range:request.headers}});
    if (!object) return new Response('Not Found', {{status:404}});
    const headers = new Headers(); object.writeHttpMetadata(headers); headers.set('content-type', key.endsWith('.png') ? 'image/png' : 'audio/webm'); headers.set('etag', object.httpEtag); headers.set('accept-ranges','bytes'); headers.set('cache-control','public, max-age=31536000, immutable'); headers.set('x-content-type-options','nosniff');
    if (object.range) {{ const offset=object.range.offset??0; const length=object.range.length??object.size; headers.set('content-range',`bytes ${{offset}}-${{offset+length-1}}/${{object.size}}`); }}
    return new Response(request.method === 'HEAD' ? null : object.body, {{status:object.range?206:200,headers}});
  }}
}};"#
    )
}

fn decode<T: for<'de> Deserialize<'de>>(
    response: reqwest::blocking::Response,
) -> AppResult<Envelope<T>> {
    let status = response.status();
    let value: Envelope<T> = response.json().map_err(http_error)?;
    if !status.is_success() || !value.success {
        return Err(api_error(
            status.as_u16(),
            value.errors.as_deref().unwrap_or(&[]),
        ));
    }
    Ok(value)
}

fn decode_value(response: reqwest::blocking::Response) -> AppResult<Value> {
    let status = response.status();
    let value: Value = response.json().map_err(http_error)?;
    if !status.is_success() || value.get("success").and_then(Value::as_bool) == Some(false) {
        let errors = serde_json::from_value::<Vec<ApiMessage>>(
            value.get("errors").cloned().unwrap_or_default(),
        )
        .unwrap_or_default();
        return Err(api_error(status.as_u16(), &errors));
    }
    Ok(value)
}

fn api_error(status: u16, errors: &[ApiMessage]) -> AppError {
    let details = errors
        .iter()
        .map(|value| {
            format!(
                "{}: {}",
                value
                    .code
                    .map_or_else(|| "Cloudflare".into(), |code| code.to_string()),
                value.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let code = match status {
        401 => "cloudflare_token_invalid",
        403 => "cloudflare_permission",
        409 => "cloudflare_conflict",
        429 => "cloudflare_rate_limited",
        _ => "cloudflare_api",
    };
    AppError::with_details(code, "Cloudflare could not complete the request.", details)
}

fn http_error(error: impl std::fmt::Display) -> AppError {
    AppError::with_details(
        "cloudflare_network",
        "Cloudflare could not be reached.",
        error.to_string(),
    )
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        database::ProjectSession,
        domain::{
            CreateProjectRequest, EntryForm, EntryRelation, EntrySortMode, EntrySortSettingsV2,
            EntrySortSource, Example, ExampleForm, ManualSortItem, ManualSortLayoutV1,
            SaveEntryRequest, Sense, UpdateProjectSettingsRequest, WritingSystem,
        },
    };
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Mutex, atomic::AtomicUsize},
    };
    use tempfile::tempdir;

    fn mock_cloudflare(
        responses: Vec<String>,
    ) -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let server_count = Arc::clone(&count);
        let handle = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request);
                server_count.fetch_add(1, Ordering::SeqCst);
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{address}"), count, handle)
    }

    fn response(status: &str, body: &str, extra_headers: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{extra_headers}\r\n{body}",
            body.len()
        )
    }

    fn capture_request(response: String) -> (String, Arc<Mutex<String>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        let server_capture = Arc::clone(&captured);
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            let expected = loop {
                let read = stream.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..read]);
                if let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n")
                {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().to_owned())
                        })
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0);
                    break header_end + 4 + length;
                }
            };
            while request.len() < expected {
                let read = stream.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..read]);
            }
            *server_capture.lock().unwrap() = String::from_utf8_lossy(&request).into_owned();
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}"), captured, handle)
    }

    #[test]
    fn resource_names_are_stable_and_valid() {
        assert_eq!(
            resource_stem("德固達 Test!", "550bec18-116e-4abf"),
            "bkuw-test-550bec18"
        );
        assert_eq!(media_bucket_name("dictionary"), "dictionary-media");
        assert_eq!(
            validate_resource_name(&"a".repeat(59), 58, "publish_worker_name_invalid")
                .unwrap_err()
                .code,
            "publish_worker_name_invalid"
        );
    }

    #[test]
    fn markdown_escapes_html_and_rejects_unsafe_links() {
        let html = markdown_to_html("# About\n\n<script>x</script>\n\n[site](https://example.com)")
            .unwrap();
        assert!(html.contains("<h1>About</h1>"));
        assert!(!html.contains("<script>"));
        assert!(markdown_to_html("[bad](javascript:alert(1))").is_err());
        assert!(markdown_to_html("[bad][link]\n\n[link]: javascript:alert(1)").is_err());
    }

    #[test]
    fn token_url_has_only_required_publish_permissions() {
        let url = token_template_url("0123456789abcdef0123456789abcdef").unwrap();
        assert!(url.contains("workers_scripts"));
        assert!(url.contains("workers_r2"));
        assert!(!url.contains("workers_routes"));
    }

    #[test]
    fn cloudflare_client_retries_429_without_exposing_the_token() {
        let throttled =
            r#"{"success":false,"result":null,"errors":[{"code":429,"message":"wait"}]}"#;
        let ready = r#"{"success":true,"result":{"buckets":[]},"errors":null}"#;
        let (root, count, server) = mock_cloudflare(vec![
            response("429 Too Many Requests", throttled, "Retry-After: 0\r\n"),
            response("200 OK", ready, ""),
        ]);
        let client = CloudflareClient::new_with_root("account", "secret-token", root).unwrap();
        assert!(client.list_buckets().unwrap().is_empty());
        server.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn existing_bucket_without_matching_owner_marker_is_rejected() {
        let buckets =
            r#"{"success":true,"result":{"buckets":[{"name":"owned-name"}]},"errors":null}"#;
        let (root, _, server) = mock_cloudflare(vec![
            response("200 OK", buckets, ""),
            response("404 Not Found", "{}", ""),
        ]);
        let client = CloudflareClient::new_with_root("account", "token", root).unwrap();
        assert_eq!(
            client
                .ensure_bucket("owned-name", "project")
                .unwrap_err()
                .code,
            "publish_bucket_owned"
        );
        server.join().unwrap();
    }

    #[test]
    fn worker_with_another_project_annotation_is_rejected() {
        let settings = r#"{"success":true,"result":{"annotations":{"workers/tag":"bkuw:another"}},"errors":null}"#;
        let (root, _, server) = mock_cloudflare(vec![response("200 OK", settings, "")]);
        let client = CloudflareClient::new_with_root("account", "token", root).unwrap();
        assert_eq!(
            client
                .ensure_worker_available("worker", "project")
                .unwrap_err()
                .code,
            "publish_worker_owned"
        );
        server.join().unwrap();
    }

    #[test]
    fn missing_worker_is_reserved_for_the_project_before_assets_upload() {
        let missing =
            r#"{"success":false,"result":null,"errors":[{"code":10090,"message":"not found"}]}"#;
        let created = r#"{"success":true,"result":{"etag":"version-1"},"errors":null}"#;
        let (root, count, server) = mock_cloudflare(vec![
            response("404 Not Found", missing, ""),
            response("200 OK", created, ""),
        ]);
        let client = CloudflareClient::new_with_root("account", "token", root).unwrap();
        assert!(!client.ensure_worker_available("worker", "project").unwrap());
        client.create_owned_worker("worker", "project").unwrap();
        server.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn worker_module_upload_includes_the_main_module_filename() {
        let created = r#"{"success":true,"result":{"etag":"version-1"},"errors":null}"#;
        let (root, captured, server) = capture_request(response("200 OK", created, ""));
        let client = CloudflareClient::new_with_root("account", "token", root).unwrap();
        client.create_owned_worker("worker", "project").unwrap();
        server.join().unwrap();
        let request = captured.lock().unwrap();
        assert!(request.contains("name=\"main.js\"; filename=\"main.js\""));
        assert!(request.contains("application/javascript+module"));
    }

    #[test]
    fn r2_object_upload_sends_the_object_as_the_raw_request_body() {
        let uploaded = r#"{"success":true,"result":{"key":".bkuw/owner.json"},"errors":null}"#;
        let (root, captured, server) = capture_request(response("200 OK", uploaded, ""));
        let client = CloudflareClient::new_with_root("account", "token", root).unwrap();
        let body = br#"{"schemaVersion":1,"projectId":"project"}"#;
        client
            .upload_bytes("bucket", ".bkuw/owner.json", body, "application/json")
            .unwrap();
        server.join().unwrap();
        let request = captured.lock().unwrap();
        let (_, request_body) = request.split_once("\r\n\r\n").unwrap();
        assert!(
            request
                .to_ascii_lowercase()
                .contains("content-type: application/json")
        );
        assert!(!request.to_ascii_lowercase().contains("multipart/form-data"));
        assert_eq!(request_body.as_bytes(), body);
    }

    #[test]
    fn corpus_contract_filters_private_fields_and_keeps_ownership_invariants() {
        let directory = tempdir().unwrap();
        let mut session = ProjectSession::create(CreateProjectRequest {
            parent_dir: directory.path().to_string_lossy().into_owned(),
            name: "Publish Contract".into(),
            language_name: Some("Test".into()),
            language_code: Some("tst".into()),
        })
        .unwrap();
        let snapshot = session.snapshot().unwrap();
        let primary = snapshot.writing_systems[0].clone();
        let ipa = WritingSystem {
            id: "ws-ipa".into(),
            name: "IPA".into(),
            kind: "phonetic".into(),
            script_code: Some("Latn".into()),
            language_tag: None,
            display_role: None,
            sort_order: 1,
            font_family: None,
            notes: None,
        };
        session
            .update_settings(UpdateProjectSettingsRequest {
                name: snapshot.project.name,
                language_name: snapshot.project.language_name,
                language_code: snapshot.project.language_code,
                analysis_language: Some("en".into()),
                description: None,
                writing_systems: vec![primary.clone(), ipa.clone()],
                part_of_speech_options: vec!["verb".into()],
                semantic_domain_options: vec!["motion".into()],
            })
            .unwrap();

        let mut target = session.create_entry().unwrap();
        target.forms.push(EntryForm {
            id: "target-form".into(),
            writing_system_id: primary.id.clone(),
            text: "guò".into(),
            variant_label: None,
            dialect: None,
            status: None,
            notes: None,
            sort_order: 0,
        });
        target.senses.push(Sense {
            id: "target-sense".into(),
            gloss: Some("cross".into()),
            definition: None,
            part_of_speech: Some("verb".into()),
            semantic_domain: Some("motion".into()),
            sort_order: 0,
            examples: Vec::new(),
        });
        let target = session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: target,
            })
            .unwrap();

        let mut source = session.create_entry().unwrap();
        source.notes = Some("private entry note".into());
        source.forms.extend([
            EntryForm {
                id: "source-form".into(),
                writing_system_id: primary.id.clone(),
                text: "guò".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 0,
            },
            EntryForm {
                id: "source-ipa".into(),
                writing_system_id: ipa.id.clone(),
                text: "kuɔ".into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 1,
            },
        ]);
        source.senses.push(Sense {
            id: "source-sense".into(),
            gloss: Some("pass".into()),
            definition: Some("move across".into()),
            part_of_speech: Some("verb".into()),
            semantic_domain: Some("motion".into()),
            sort_order: 0,
            examples: vec![Example {
                id: "example".into(),
                translation: Some("cross it".into()),
                notes: Some("private example note".into()),
                sort_order: 0,
                forms: vec![ExampleForm {
                    id: "example-form".into(),
                    writing_system_id: primary.id.clone(),
                    text: "guò".into(),
                    sort_order: 0,
                }],
            }],
        });
        source.relations.push(EntryRelation {
            id: "relation".into(),
            target_entry_id: Some(target.id),
            relation_type: "root".into(),
            fallback_text: Some("guò".into()),
            notes: None,
            sort_order: 0,
        });
        session
            .save_entry(SaveEntryRequest {
                expected_revision: 0,
                entry: source,
            })
            .unwrap();

        let mut settings = session.load_publish_settings().unwrap();
        settings.writing_system_ids = vec![primary.id];
        let publication = prepare(&session.publish_snapshot().unwrap(), &settings).unwrap();
        let corpus: Value =
            serde_json::from_slice(&publication.assets["/data/corpus.json"]).unwrap();
        let entries = corpus["entries"].as_array().unwrap();
        assert_ne!(entries[0]["slug"], entries[1]["slug"]);
        let published_source = entries
            .iter()
            .find(|entry| entry["senses"][0]["gloss"] == "pass")
            .unwrap();
        assert_eq!(published_source["forms"].as_array().unwrap().len(), 1);
        assert!(published_source["relations"].as_array().unwrap().is_empty());
        let sense = &published_source["senses"][0];
        assert_eq!(sense["partOfSpeech"], "verb");
        assert!(published_source.get("partOfSpeech").is_none());
        assert!(sense["examples"][0]["notes"].is_null());

        settings.include_entry_notes = true;
        settings.include_example_notes = true;
        settings.include_relations = true;
        let publication = prepare(&session.publish_snapshot().unwrap(), &settings).unwrap();
        let corpus: Value =
            serde_json::from_slice(&publication.assets["/data/corpus.json"]).unwrap();
        let source = corpus["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["notes"] == "private entry note")
            .unwrap();
        assert_eq!(
            source["senses"][0]["examples"][0]["notes"],
            "private example note"
        );
        assert_eq!(source["relations"][0]["targetHeadword"], "guò");
    }

    #[test]
    fn corpus_preserves_project_manual_order_and_section_labels() {
        let directory = tempdir().unwrap();
        let mut session = ProjectSession::create(CreateProjectRequest {
            parent_dir: directory.path().to_string_lossy().into_owned(),
            name: "Manual Publication Order".into(),
            language_name: Some("Test".into()),
            language_code: Some("tst".into()),
        })
        .unwrap();
        let primary_id = session.snapshot().unwrap().writing_systems[0].id.clone();
        let mut ids = Vec::new();
        for text in ["alpha", "zeta"] {
            let mut entry = session.create_entry().unwrap();
            ids.push(entry.id.clone());
            entry.forms.push(EntryForm {
                id: format!("form-{text}"),
                writing_system_id: primary_id.clone(),
                text: text.into(),
                variant_label: None,
                dialect: None,
                status: None,
                notes: None,
                sort_order: 0,
            });
            entry.senses.push(Sense {
                id: format!("sense-{text}"),
                gloss: Some(text.into()),
                definition: None,
                part_of_speech: None,
                semantic_domain: None,
                sort_order: 0,
                examples: Vec::new(),
            });
            session
                .save_entry(SaveEntryRequest {
                    expected_revision: 0,
                    entry,
                })
                .unwrap();
        }
        session
            .save_manual_sort_layout(ManualSortLayoutV1 {
                version: 1,
                items: vec![
                    ManualSortItem::Heading {
                        id: "featured".into(),
                        label: "Featured".into(),
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
            .unwrap();
        session
            .save_entry_sort_settings(EntrySortSettingsV2 {
                version: 2,
                mode: EntrySortMode::Manual,
                source: EntrySortSource::WritingSystem,
                writing_system_id: primary_id,
                alphabet: Vec::new(),
            })
            .unwrap();

        let publication = prepare(
            &session.publish_snapshot().unwrap(),
            &session.load_publish_settings().unwrap(),
        )
        .unwrap();
        let corpus: Value =
            serde_json::from_slice(&publication.assets["/data/corpus.json"]).unwrap();
        let entries = corpus["entries"].as_array().unwrap();
        assert_eq!(entries[0]["primaryForm"], "zeta");
        assert_eq!(entries[0]["sectionLabel"], "Featured");
        assert_eq!(entries[1]["primaryForm"], "alpha");
        assert_eq!(entries[1]["sectionLabel"], "Regular");
    }
}
