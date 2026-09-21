use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Manager, State, ipc::Channel};

use crate::{
    database::ProjectSession,
    domain::{
        AttachSenseImageRequest, CloudflareConnectionStatus, ConnectCloudflareRequest,
        CreateProjectFromCsvRequest, CreateProjectRequest, CsvDelimiter, CsvImportPreview,
        CsvImportResult, CsvInspection, CsvPreviewRequest, DeleteEntryRequest, DeletedEntry,
        EntrySortSettingsV2, EntrySummary, ExportKind, ExportPreview, ExportProjectRequest,
        ExportResult, ExportSettingsV1, FontInstallProgress, FontPackStatus, LexicalEntry,
        ManualSortLayoutV1, ProjectSnapshot, PublishPreview, PublishProgress, PublishRequest,
        PublishResult, PublishSettingsV1, PublishState, RemoveSenseImageRequest, SaveEntryRequest,
        SenseImage, SenseImageContent, SenseImageMutation, TexEngineStatus,
        UpdateProjectSettingsRequest,
    },
    error::{AppError, AppResult},
};

fn font_manager(app: &AppHandle) -> AppResult<crate::font_manager::FontManager> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| {
            AppError::with_details(
                "font_filesystem",
                "The bkuw font cache directory is unavailable.",
                error.to_string(),
            )
        })?
        .join("fonts");
    Ok(crate::font_manager::FontManager::new(root))
}

#[derive(Default)]
pub struct AppState {
    session: Mutex<Option<ProjectSession>>,
    latex_installer: crate::export::environment::InstallerState,
    publish_runtime: crate::publish::PublishRuntime,
}

#[tauri::command]
pub fn get_publish_state(state: State<'_, AppState>) -> AppResult<PublishState> {
    let guard = active_session(&state)?;
    let session = guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
    let settings = session.load_publish_settings()?;
    let deployment = session.load_publish_deployment()?;
    let connection = crate::publish::connection_status(
        &state.publish_runtime,
        deployment.as_ref().map(|value| value.account_id.as_str()),
        deployment
            .as_ref()
            .map(|value| value.workers_subdomain.clone()),
    );
    Ok(PublishState {
        settings,
        deployment,
        connection,
    })
}

#[tauri::command]
pub fn get_cloudflare_token_url(account_id: String) -> AppResult<String> {
    crate::publish::token_template_url(account_id.trim())
}

#[tauri::command]
pub async fn connect_cloudflare(
    app: AppHandle,
    request: ConnectCloudflareRequest,
) -> AppResult<CloudflareConnectionStatus> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        crate::publish::connect(
            &state.publish_runtime,
            request.account_id.trim(),
            request.api_token.trim(),
            request.requested_subdomain.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub fn disconnect_cloudflare(state: State<'_, AppState>, account_id: String) {
    state.publish_runtime.disconnect(account_id.trim());
}

#[tauri::command]
pub fn save_publish_settings(
    state: State<'_, AppState>,
    settings: PublishSettingsV1,
) -> AppResult<PublishSettingsV1> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .save_publish_settings(settings)
}

#[tauri::command]
pub async fn preview_publish(app: AppHandle) -> AppResult<PublishPreview> {
    let (snapshot, settings, account_id) = {
        let state = app.state::<AppState>();
        let guard = active_session(&state)?;
        let session = guard
            .as_ref()
            .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
        let deployment = session.load_publish_deployment()?;
        (
            session.publish_snapshot()?,
            session.load_publish_settings()?,
            deployment
                .map(|value| value.account_id)
                .or_else(|| state.publish_runtime.active_account()),
        )
    };
    run_blocking(move || {
        let state = app.state::<AppState>();
        crate::publish::preview(
            &snapshot,
            &settings,
            &state.publish_runtime,
            account_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub async fn publish_site(
    app: AppHandle,
    request: PublishRequest,
    on_progress: Channel<PublishProgress>,
) -> AppResult<PublishResult> {
    let (snapshot, settings, account_id, project_id) = {
        let state = app.state::<AppState>();
        let guard = active_session(&state)?;
        let session = guard
            .as_ref()
            .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
        let snapshot = session.publish_snapshot()?;
        let project_id = snapshot.export.project.id.clone();
        let deployment = session.load_publish_deployment()?;
        let account_id = deployment
            .as_ref()
            .map(|value| value.account_id.clone())
            .or_else(|| state.publish_runtime.active_account())
            .ok_or_else(|| {
                AppError::new(
                    "cloudflare_not_connected",
                    "Connect Cloudflare before publishing.",
                )
            })?;
        (
            snapshot,
            session.load_publish_settings()?,
            account_id,
            project_id,
        )
    };
    let app_for_publish = app.clone();
    let (result, saved_deployment) = run_blocking(move || {
        let state = app_for_publish.state::<AppState>();
        crate::publish::publish(
            &snapshot,
            &settings,
            &account_id,
            &request.snapshot_token,
            &state.publish_runtime,
            &on_progress,
        )
    })
    .await?;
    let state = app.state::<AppState>();
    let mut guard = active_session(&state)?;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
    if session.snapshot()?.project.id != project_id {
        return Err(AppError::new(
            "project_changed",
            "Another project was opened while publishing.",
        ));
    }
    session.save_publish_deployment(&saved_deployment)?;
    Ok(result)
}

#[tauri::command]
pub fn cancel_publish(state: State<'_, AppState>) {
    state.publish_runtime.cancel();
}

#[tauri::command]
pub async fn retry_publish_cleanup(app: AppHandle) -> AppResult<usize> {
    let (snapshot, settings, deployment, project_id) = {
        let state = app.state::<AppState>();
        let guard = active_session(&state)?;
        let session = guard
            .as_ref()
            .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
        let deployment = session.load_publish_deployment()?.ok_or_else(|| {
            AppError::new(
                "publish_not_deployed",
                "This project has not been published.",
            )
        })?;
        let snapshot = session.publish_snapshot()?;
        let project_id = snapshot.export.project.id.clone();
        (
            snapshot,
            session.load_publish_settings()?,
            deployment,
            project_id,
        )
    };
    let app_for_cleanup = app.clone();
    let deleted = run_blocking(move || {
        let state = app_for_cleanup.state::<AppState>();
        crate::publish::retry_cleanup_for_snapshot(
            &state.publish_runtime,
            &deployment,
            &snapshot,
            &settings,
        )
    })
    .await?;
    let state = app.state::<AppState>();
    let mut guard = active_session(&state)?;
    if guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .snapshot()?
        .project
        .id
        != project_id
    {
        return Err(AppError::new(
            "project_changed",
            "Another project was opened while cleaning website media.",
        ));
    }
    if let Some(mut deployment) = guard
        .as_ref()
        .and_then(|session| session.load_publish_deployment().ok().flatten())
    {
        deployment.cleanup_pending = false;
        guard
            .as_mut()
            .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
            .save_publish_deployment(&deployment)?;
    }
    Ok(deleted)
}

fn lock_state<'a>(
    state: &'a State<'_, AppState>,
) -> AppResult<MutexGuard<'a, Option<ProjectSession>>> {
    state
        .session
        .lock()
        .map_err(|_| AppError::new("internal", "The project session is unavailable."))
}

fn active_session<'a>(
    state: &'a State<'_, AppState>,
) -> AppResult<MutexGuard<'a, Option<ProjectSession>>> {
    let guard = lock_state(state)?;
    if guard.is_none() {
        return Err(AppError::new("no_project", "No project is currently open."));
    }
    Ok(guard)
}

async fn run_blocking<T, F>(task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| {
            AppError::with_details(
                "internal",
                "The background task could not complete.",
                error.to_string(),
            )
        })?
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    request: CreateProjectRequest,
) -> AppResult<ProjectSnapshot> {
    let mut guard = lock_state(&state)?;
    if guard.is_some() {
        return Err(AppError::new(
            "project_open",
            "Close the current project before creating another one.",
        ));
    }
    let session = ProjectSession::create(request)?;
    let snapshot = session.snapshot()?;
    *guard = Some(session);
    Ok(snapshot)
}

#[tauri::command]
pub async fn inspect_csv(
    path: String,
    delimiter: Option<CsvDelimiter>,
) -> AppResult<CsvInspection> {
    run_blocking(move || crate::csv_import::inspect(&path, delimiter)).await
}

#[tauri::command]
pub async fn preview_csv_import(request: CsvPreviewRequest) -> AppResult<CsvImportPreview> {
    run_blocking(move || crate::csv_import::preview(&request)).await
}

#[tauri::command]
pub async fn create_project_from_csv(
    app: AppHandle,
    request: CreateProjectFromCsvRequest,
) -> AppResult<CsvImportResult> {
    {
        let state = app.state::<AppState>();
        if lock_state(&state)?.is_some() {
            return Err(AppError::new(
                "project_open",
                "Close the current project before creating another one.",
            ));
        }
    }
    let (session, result) = run_blocking(move || crate::csv_import::create(request)).await?;
    let state = app.state::<AppState>();
    let mut guard = lock_state(&state)?;
    if guard.is_some() {
        return Err(AppError::new(
            "project_open",
            "Another project was opened while the CSV import was running.",
        ));
    }
    *guard = Some(session);
    Ok(result)
}

#[tauri::command]
pub fn open_project(state: State<'_, AppState>, path: String) -> AppResult<ProjectSnapshot> {
    let mut guard = lock_state(&state)?;
    if guard.is_some() {
        return Err(AppError::new(
            "project_open",
            "Close the current project before opening another one.",
        ));
    }
    let session = ProjectSession::open(path)?;
    let snapshot = session.snapshot()?;
    *guard = Some(session);
    Ok(snapshot)
}

#[tauri::command]
pub fn close_project(state: State<'_, AppState>) -> AppResult<()> {
    let mut guard = lock_state(&state)?;
    if let Some(session) = guard.take() {
        session.close()?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_project_snapshot(state: State<'_, AppState>) -> AppResult<ProjectSnapshot> {
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .snapshot()
}

#[tauri::command]
pub fn update_project_settings(
    state: State<'_, AppState>,
    request: UpdateProjectSettingsRequest,
) -> AppResult<ProjectSnapshot> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .update_settings(request)
}

#[tauri::command]
pub fn query_entry_summaries(
    state: State<'_, AppState>,
    query: String,
) -> AppResult<Vec<EntrySummary>> {
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .query_entries(&query)
}

#[tauri::command]
pub fn load_entry(state: State<'_, AppState>, id: String) -> AppResult<LexicalEntry> {
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .load_entry(&id)
}

#[tauri::command]
pub fn create_entry(state: State<'_, AppState>) -> AppResult<LexicalEntry> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .create_entry()
}

#[tauri::command]
pub fn save_entry(
    state: State<'_, AppState>,
    request: SaveEntryRequest,
) -> AppResult<LexicalEntry> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .save_entry(request)
}

#[tauri::command]
pub fn list_sense_images(
    state: State<'_, AppState>,
    sense_id: String,
) -> AppResult<Vec<SenseImage>> {
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .list_sense_images(&sense_id)
}

#[tauri::command]
pub fn attach_sense_image(
    state: State<'_, AppState>,
    request: AttachSenseImageRequest,
) -> AppResult<SenseImageMutation> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .attach_sense_image(request)
}

#[tauri::command]
pub fn load_sense_image(
    state: State<'_, AppState>,
    image_id: String,
) -> AppResult<SenseImageContent> {
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .load_sense_image(&image_id)
}

#[tauri::command]
pub fn remove_sense_image(
    state: State<'_, AppState>,
    request: RemoveSenseImageRequest,
) -> AppResult<SenseImageMutation> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .remove_sense_image(request)
}

#[tauri::command]
pub fn delete_entry(
    state: State<'_, AppState>,
    request: DeleteEntryRequest,
) -> AppResult<DeletedEntry> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .delete_entry(request)
}

#[tauri::command]
pub fn restore_entry(state: State<'_, AppState>, id: String) -> AppResult<LexicalEntry> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .restore_entry(&id)
}

#[tauri::command]
pub fn save_export_settings(
    state: State<'_, AppState>,
    settings: ExportSettingsV1,
) -> AppResult<ExportSettingsV1> {
    let mut guard = active_session(&state)?;
    guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .save_export_settings(settings.clone())?;
    Ok(settings)
}

#[tauri::command]
pub fn save_entry_sort_settings(
    state: State<'_, AppState>,
    settings: EntrySortSettingsV2,
) -> AppResult<ProjectSnapshot> {
    let mut guard = active_session(&state)?;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
    session.save_entry_sort_settings(settings)?;
    session.snapshot()
}

#[tauri::command]
pub fn save_manual_sort_layout(
    state: State<'_, AppState>,
    layout: ManualSortLayoutV1,
) -> AppResult<ProjectSnapshot> {
    let mut guard = active_session(&state)?;
    let session = guard
        .as_mut()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?;
    session.save_manual_sort_layout(layout)?;
    session.snapshot()
}

#[tauri::command]
pub async fn preview_export(app: AppHandle, kind: ExportKind) -> AppResult<ExportPreview> {
    run_blocking(move || {
        let snapshot = {
            let state = app.state::<AppState>();
            let guard = active_session(&state)?;
            guard
                .as_ref()
                .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
                .export_snapshot()?
        };
        let fonts = font_manager(&app)?;
        crate::export::preview(&snapshot, kind, Some(&fonts))
    })
    .await
}

#[tauri::command]
pub async fn export_project(
    app: AppHandle,
    request: ExportProjectRequest,
) -> AppResult<ExportResult> {
    run_blocking(move || {
        let snapshot = {
            let state = app.state::<AppState>();
            let guard = active_session(&state)?;
            guard
                .as_ref()
                .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
                .export_snapshot()?
        };
        let fonts = font_manager(&app)?;
        crate::export::run(&snapshot, request, Some(&fonts))
    })
    .await
}

#[tauri::command]
pub async fn check_latex_environment(
    app: AppHandle,
) -> AppResult<crate::export::environment::EnvironmentStatus> {
    run_blocking(move || {
        let snapshot = {
            let state = app.state::<AppState>();
            let guard = active_session(&state)?;
            guard
                .as_ref()
                .ok_or_else(|| AppError::new("no_project", "No project is open."))?
                .export_snapshot()?
        };
        let root = app.path().app_local_data_dir().map_err(|_| {
            AppError::new(
                "latex_environment",
                "The app data directory is unavailable.",
            )
        })?;
        crate::export::check_environment(&snapshot, &font_manager(&app)?, &root.join("diagnostics"))
    })
    .await
}

#[tauri::command]
pub async fn install_latex(
    app: AppHandle,
    on_progress: Channel<crate::export::environment::InstallProgress>,
) -> AppResult<()> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let root = app.path().app_local_data_dir().map_err(|_| {
            AppError::new(
                "latex_install_filesystem",
                "The app data directory is unavailable.",
            )
        })?;
        crate::export::environment::install(
            &root.join("latex-installers"),
            &state.latex_installer,
            &|progress| {
                let _ = on_progress.send(progress);
            },
        )
    })
    .await
}

#[tauri::command]
pub fn cancel_latex_download(state: State<'_, AppState>) {
    state.latex_installer.cancel();
}

#[tauri::command]
pub async fn save_latex_install_guide(destination: String) -> AppResult<String> {
    run_blocking(move || crate::export::environment::save_guide(std::path::Path::new(&destination)))
        .await
}

#[tauri::command]
pub async fn detect_xelatex() -> AppResult<TexEngineStatus> {
    run_blocking(|| Ok(crate::export::detect_xelatex())).await
}

#[tauri::command]
pub async fn list_font_packs(app: AppHandle) -> AppResult<Vec<FontPackStatus>> {
    run_blocking(move || Ok(font_manager(&app)?.statuses())).await
}

#[tauri::command]
pub async fn install_font_pack(app: AppHandle, pack_id: String) -> AppResult<FontPackStatus> {
    let manager = font_manager(&app)?;
    tauri::async_runtime::spawn_blocking(move || manager.install(&pack_id))
        .await
        .map_err(|error| {
            AppError::with_details(
                "internal",
                "The font installation task could not complete.",
                error.to_string(),
            )
        })?
}

#[tauri::command]
pub async fn install_font_packs(
    app: AppHandle,
    pack_ids: Vec<String>,
    on_progress: Channel<FontInstallProgress>,
) -> AppResult<Vec<FontPackStatus>> {
    let manager = font_manager(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let pack_count = pack_ids.len();
        let mut installed = Vec::with_capacity(pack_count);
        for (pack_index, pack_id) in pack_ids.into_iter().enumerate() {
            let report = |phase: &str, downloaded_bytes: u64, total_bytes: Option<u64>| {
                let _ = on_progress.send(FontInstallProgress {
                    pack_id: pack_id.clone(),
                    phase: phase.into(),
                    pack_index,
                    pack_count,
                    downloaded_bytes,
                    total_bytes,
                });
            };
            report("downloading", 0, None);
            let result = manager.install_with_progress(&pack_id, &|downloaded, total| {
                report(
                    if total == Some(downloaded) {
                        "verifying"
                    } else {
                        "downloading"
                    },
                    downloaded,
                    total,
                );
            });
            match result {
                Ok(status) => {
                    report(
                        "installed",
                        status.installed_bytes,
                        Some(status.installed_bytes),
                    );
                    installed.push(status);
                }
                Err(error) => {
                    report("failed", 0, None);
                    return Err(error);
                }
            }
        }
        Ok(installed)
    })
    .await
    .map_err(|error| {
        AppError::with_details(
            "internal",
            "The font installation task could not complete.",
            error.to_string(),
        )
    })?
}

#[tauri::command]
pub fn list_audio(
    state: State<'_, AppState>,
    owner: crate::domain::AudioOwner,
) -> AppResult<Vec<crate::domain::AudioAttachment>> {
    active_session(&state)?
        .as_ref()
        .expect("checked")
        .list_audio(&owner)
}

#[tauri::command]
pub async fn import_audio(
    app: AppHandle,
    request: crate::domain::ImportAudioRequest,
) -> AppResult<crate::domain::AudioMutation> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let token = active_session(&state)?
            .as_ref()
            .expect("checked")
            .audio_import_token(&request)?;
        let resources = app
            .path()
            .resource_dir()
            .map_err(|_| AppError::new("audio_tools", "Audio resources are unavailable."))?;
        let tools =
            crate::database::audio::AudioTools::bundled(&resources.join("resources/audio"))?;
        let prepared =
            crate::database::audio::prepare(&tools, std::path::Path::new(&request.source_path))?;
        active_session(&state)?
            .as_mut()
            .expect("checked")
            .attach_audio(request, &token, prepared)
    })
    .await
}

#[tauri::command]
pub async fn load_audio(
    app: AppHandle,
    audio_id: String,
) -> AppResult<crate::domain::AudioContent> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        active_session(&state)?
            .as_ref()
            .expect("checked")
            .load_audio(&audio_id)
    })
    .await
}

#[tauri::command]
pub fn begin_audio_recording(
    state: State<'_, AppState>,
    request: crate::domain::ImportAudioRequest,
) -> AppResult<String> {
    active_session(&state)?
        .as_ref()
        .expect("checked")
        .audio_import_token(&request)
}

#[tauri::command]
pub async fn save_audio_recording(
    app: AppHandle,
    request: crate::domain::RecordingRequest,
) -> AppResult<crate::domain::AudioMutation> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        let attachment = crate::domain::ImportAudioRequest {
            entry_id: request.entry_id.clone(),
            owner: request.owner.clone(),
            expected_revision: request.expected_revision,
            source_path: String::new(),
        };
        let token = active_session(&state)?
            .as_ref()
            .expect("checked")
            .audio_import_token(&attachment)?;
        if token != request.session_token {
            return Err(AppError::new(
                "audio_stale",
                "The project changed during recording.",
            ));
        }
        let resources = app
            .path()
            .resource_dir()
            .map_err(|_| AppError::new("audio_tools", "Audio resources are unavailable."))?;
        let tools =
            crate::database::audio::AudioTools::bundled(&resources.join("resources/audio"))?;
        let prepared = crate::database::audio::prepare_recording(&tools, &request)?;
        active_session(&state)?
            .as_mut()
            .expect("checked")
            .attach_audio(attachment, &token, prepared)
    })
    .await
}

#[tauri::command]
pub fn remove_audio(
    state: State<'_, AppState>,
    request: crate::domain::RemoveAudioRequest,
) -> AppResult<crate::domain::AudioMutation> {
    active_session(&state)?
        .as_mut()
        .expect("checked")
        .remove_audio(request)
}
