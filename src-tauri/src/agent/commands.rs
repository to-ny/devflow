use std::sync::RwLock;

use log::{error, info};
use tauri::{AppHandle, Emitter, State};

use super::service::AgentState;
use super::types::{AgentStatus, AgentStatusPayload, ChatMessage};

#[tauri::command]
pub async fn agent_send_message(
    app_handle: AppHandle,
    state: State<'_, RwLock<AgentState>>,
    project_path: String,
    messages: Vec<ChatMessage>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    // Use read lock to check state, then write lock to initialize and start
    let (adapter, cancel_token) = {
        // First, check with read lock
        let needs_work = {
            let state_guard = state.read().map_err(|e| e.to_string())?;
            if state_guard.is_running {
                return Err("Agent is already processing a request".to_string());
            }
            state_guard.project_path.as_ref() != Some(&project_path)
                || state_guard.adapter.is_none()
        };

        // If we need to modify state, get write lock
        let mut state_guard = state.write().map_err(|e| e.to_string())?;

        // Re-check is_running under write lock to prevent race
        if state_guard.is_running {
            return Err("Agent is already processing a request".to_string());
        }

        if needs_work {
            state_guard.initialize(&project_path).map_err(|e| {
                error!("Failed to initialize agent adapter: {}", e);
                e.to_string()
            })?;
        }

        let adapter = state_guard
            .get_adapter()
            .ok_or_else(|| "Agent not initialized".to_string())?;
        let token = state_guard.start_run();
        (adapter, token)
    };

    let result = adapter
        .send_message(messages, system_prompt, app_handle, cancel_token)
        .await;

    // Mark run as finished
    {
        let mut state_guard = state.write().map_err(|e| e.to_string())?;
        state_guard.finish_run();
    }

    result.map_err(|e| {
        error!("Failed to send message: {}", e);
        e.to_string()
    })
}

#[tauri::command]
pub fn agent_cancel(
    app_handle: AppHandle,
    state: State<'_, RwLock<AgentState>>,
) -> Result<(), String> {
    let mut state_guard = state.write().map_err(|e| e.to_string())?;

    if !state_guard.is_running {
        return Ok(()); // Nothing to cancel
    }

    info!("Cancelling agent operation");
    state_guard.cancel();

    // Emit status update
    let _ = app_handle.emit(
        "agent-status",
        AgentStatusPayload::new(AgentStatus::Cancelled, None),
    );

    Ok(())
}

#[tauri::command]
pub fn agent_is_running(state: State<'_, RwLock<AgentState>>) -> Result<bool, String> {
    let state_guard = state.read().map_err(|e| e.to_string())?;
    Ok(state_guard.is_running)
}

#[tauri::command]
pub fn agent_clear_state(state: State<'_, RwLock<AgentState>>) -> Result<(), String> {
    let mut state_guard = state.write().map_err(|e| e.to_string())?;
    state_guard.clear();
    Ok(())
}
