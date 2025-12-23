use std::sync::Mutex;

use tauri::{AppHandle, State};

use super::WatcherService;

pub struct WatcherState(pub Mutex<Option<WatcherService>>);

#[tauri::command]
pub fn watcher_start(
    app_handle: AppHandle,
    state: State<'_, WatcherState>,
    project_path: String,
) -> Result<(), String> {
    let mut watcher = state.0.lock().map_err(|e| e.to_string())?;
    *watcher = Some(WatcherService::new(app_handle, &project_path)?);
    Ok(())
}

#[tauri::command]
pub fn watcher_stop(state: State<'_, WatcherState>) -> Result<(), String> {
    let mut watcher = state.0.lock().map_err(|e| e.to_string())?;
    *watcher = None;
    Ok(())
}
