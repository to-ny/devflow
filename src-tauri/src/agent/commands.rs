use std::sync::Mutex;

use log::error;
use tauri::{AppHandle, State};

use super::service::AgentState;
use super::types::ChatMessage;

#[tauri::command]
pub async fn agent_send_message(
    app_handle: AppHandle,
    state: State<'_, Mutex<AgentState>>,
    project_path: String,
    messages: Vec<ChatMessage>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    let needs_init = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.project_path.as_ref() != Some(&project_path) || state.adapter.is_none()
    };

    if needs_init {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        state.initialize(&project_path).map_err(|e| {
            error!("Failed to initialize agent adapter: {}", e);
            e.to_string()
        })?;
    }

    let adapter = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state
            .get_adapter()
            .ok_or_else(|| "Agent not initialized".to_string())?
    };

    adapter
        .send_message(messages, system_prompt, app_handle)
        .await
        .map_err(|e| {
            error!("Failed to send message: {}", e);
            e.to_string()
        })
}

#[tauri::command]
pub fn agent_clear_state(state: State<'_, Mutex<AgentState>>) -> Result<(), String> {
    let mut state = state.lock().map_err(|e| e.to_string())?;
    state.clear();
    Ok(())
}
