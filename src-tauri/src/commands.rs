use std::sync::{Mutex, MutexGuard};

use tauri::State;

use crate::{
    database::ProjectSession,
    domain::{
        CreateProjectRequest, DeleteEntryRequest, DeletedEntry, EntrySummary, LexicalEntry,
        ProjectSnapshot, SaveEntryRequest, UpdateProjectSettingsRequest,
    },
    error::{AppError, AppResult},
};

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
