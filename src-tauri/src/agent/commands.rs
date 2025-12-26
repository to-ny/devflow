use std::sync::{PoisonError, RwLock};

use tauri::{AppHandle, Emitter, State};

use super::state::AgentState;
use super::types::{AgentStatus, AgentStatusPayload, ChatMessage};

fn lock_error<T>(_: PoisonError<T>) -> String {
    "Lock poisoned".to_string()
}

#[tauri::command]
pub async fn agent_send_message(
    app_handle: AppHandle,
    state: State<'_, RwLock<AgentState>>,
    project_path: String,
    messages: Vec<ChatMessage>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    // Use read lock to check state, then write lock to initialize and start
    let (adapter, session, cancel_token) = {
        // First, check with read lock
        let needs_reload = {
            let state_guard = state.read().map_err(lock_error)?;
            if state_guard.is_running {
                return Err("Agent is already processing a request".to_string());
            }
            state_guard.needs_reload(&project_path)
        };

        // If we need to modify state, get write lock
        let mut state_guard = state.write().map_err(lock_error)?;

        // Re-check is_running under write lock to prevent race
        if state_guard.is_running {
            return Err("Agent is already processing a request".to_string());
        }

        if needs_reload {
            state_guard
                .initialize(&project_path)
                .map_err(|e| e.to_string())?;
        }

        let adapter = state_guard
            .get_adapter()
            .ok_or_else(|| "Agent not initialized".to_string())?;
        let session = state_guard.get_session();
        let token = state_guard.start_run();
        (adapter, session, token)
    };

    let result = adapter
        .send_message(messages, system_prompt, session, app_handle, cancel_token)
        .await;

    // Mark run as finished
    {
        let mut state_guard = state.write().map_err(lock_error)?;
        state_guard.finish_run();
    }

    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agent_cancel(
    app_handle: AppHandle,
    state: State<'_, RwLock<AgentState>>,
) -> Result<(), String> {
    let mut state_guard = state.write().map_err(lock_error)?;

    if !state_guard.is_running {
        return Ok(());
    }

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
    let state_guard = state.read().map_err(lock_error)?;
    Ok(state_guard.is_running)
}

#[tauri::command]
pub fn agent_clear_state(state: State<'_, RwLock<AgentState>>) -> Result<(), String> {
    let mut state_guard = state.write().map_err(lock_error)?;
    state_guard.clear();
    Ok(())
}

#[tauri::command]
pub async fn agent_approve_plan(state: State<'_, RwLock<AgentState>>) -> Result<bool, String> {
    let session = {
        let state_guard = state.read().map_err(lock_error)?;
        state_guard.get_session()
    };

    let result = session.approve_plan().await;
    Ok(result)
}

#[tauri::command]
pub async fn agent_reject_plan(
    state: State<'_, RwLock<AgentState>>,
    reason: Option<String>,
) -> Result<bool, String> {
    let session = {
        let state_guard = state.read().map_err(lock_error)?;
        state_guard.get_session()
    };

    let result = session.reject_plan(reason).await;
    Ok(result)
}

#[tauri::command]
pub async fn agent_has_pending_plan(state: State<'_, RwLock<AgentState>>) -> Result<bool, String> {
    let session = {
        let state_guard = state.read().map_err(lock_error)?;
        state_guard.get_session()
    };

    Ok(session.has_pending_plan().await)
}
