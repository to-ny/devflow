use std::sync::{Arc, PoisonError, RwLock};

use tauri::{AppHandle, Emitter, State};

use super::memory::LoadResult;
use super::state::AgentState;
use super::types::{
    AgentStatus, AgentStatusPayload, ChatMessage, MemoryLoadedPayload, MemoryWarningPayload,
};
use super::usage::{SessionUsageTracker, UsageTotals};

fn lock_error<T>(_: PoisonError<T>) -> String {
    "Lock poisoned".to_string()
}

fn emit_memory_result(app_handle: &AppHandle, result: &LoadResult) {
    match result {
        LoadResult::Loaded {
            path,
            byte_len,
            truncated,
        } => {
            let _ = app_handle.emit(
                "memory-loaded",
                MemoryLoadedPayload {
                    path: path.clone(),
                    byte_len: *byte_len,
                    truncated: *truncated,
                },
            );
        }
        LoadResult::NotFound => {}
        LoadResult::Error(message) => {
            let _ = app_handle.emit(
                "memory-warning",
                MemoryWarningPayload {
                    message: message.clone(),
                },
            );
        }
    }
}

#[tauri::command]
pub async fn agent_send_message(
    app_handle: AppHandle,
    state: State<'_, RwLock<AgentState>>,
    usage_tracker: State<'_, Arc<SessionUsageTracker>>,
    project_path: String,
    messages: Vec<ChatMessage>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    // Use read lock to check state, then write lock to initialize and start
    let (adapter, session, cancel_token, memory) = {
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
            let load_result = state_guard
                .initialize(&project_path)
                .map_err(|e| e.to_string())?;
            emit_memory_result(&app_handle, &load_result);
        } else {
            // Check if memory file has changed
            if let Some(reload_result) = state_guard.reload_memory_if_changed() {
                emit_memory_result(&app_handle, &reload_result);
            }
        }

        let adapter = state_guard
            .get_adapter()
            .ok_or_else(|| "Agent not initialized".to_string())?;
        let session = state_guard.get_session();
        let memory = state_guard.get_memory_for_injection();
        let token = state_guard.start_run();
        (adapter, session, token, memory)
    };

    use super::provider::ExecutionContext;

    let ctx = ExecutionContext {
        session,
        cancel_token,
        usage_tracker: Arc::clone(&*usage_tracker),
    };
    let result = adapter
        .send_message(messages, system_prompt, memory, ctx, app_handle)
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

#[tauri::command]
pub fn get_session_usage(tracker: State<'_, Arc<SessionUsageTracker>>) -> UsageTotals {
    tracker.get_totals()
}

#[tauri::command]
pub fn reset_session_usage(tracker: State<'_, Arc<SessionUsageTracker>>) {
    tracker.reset();
}
