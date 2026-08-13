use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Manager, State};

use crate::{
    database::ProjectSession,
    domain::{
        AttachSenseImageRequest, CreateProjectRequest, DeleteEntryRequest, DeletedEntry,
        EntrySortSettingsV1, EntrySummary, ExportKind, ExportPreview, ExportProjectRequest,
        ExportResult, ExportSettingsV1, FontPackStatus, LexicalEntry, ManualSortLayoutV1,
        ProjectSnapshot, RemoveSenseImageRequest, SaveEntryRequest, SenseImage, SenseImageContent,
        SenseImageMutation, TexEngineStatus, UpdateProjectSettingsRequest,
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
    settings: EntrySortSettingsV1,
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
