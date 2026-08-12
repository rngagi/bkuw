use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Manager, State};

use crate::{
    database::ProjectSession,
    domain::{
        CreateProjectRequest, DeleteEntryRequest, DeletedEntry, EntrySummary, ExportKind,
        ExportPreview, ExportProjectRequest, ExportResult, ExportSettingsV1, FontPackStatus,
        LexicalEntry, ProjectSnapshot, SaveEntryRequest, TexEngineStatus,
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
pub fn preview_export(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: ExportKind,
) -> AppResult<ExportPreview> {
    let fonts = font_manager(&app)?;
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .preview_export_with_fonts(kind, &fonts)
}

#[tauri::command]
pub fn export_project(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ExportProjectRequest,
) -> AppResult<ExportResult> {
    let fonts = font_manager(&app)?;
    let guard = active_session(&state)?;
    guard
        .as_ref()
        .ok_or_else(|| AppError::new("no_project", "No project is currently open."))?
        .export_project_with_fonts(request, &fonts)
}

#[tauri::command]
pub fn detect_xelatex() -> TexEngineStatus {
    crate::export::detect_xelatex()
}

#[tauri::command]
pub fn list_font_packs(app: AppHandle) -> AppResult<Vec<FontPackStatus>> {
    Ok(font_manager(&app)?.statuses())
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
