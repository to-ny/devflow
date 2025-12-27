use std::collections::HashMap;
use std::path::Path;

use tauri::{AppHandle, Emitter};

use super::service::ConfigService;
use super::types::{ConfigChangedPayload, ProjectConfig, ProviderInfo};
use crate::agent::{get_tool_descriptions, DEFAULT_SYSTEM_PROMPT};

#[tauri::command]
pub fn config_get_last_project() -> Result<Option<String>, String> {
    let service = ConfigService::new().map_err(|e| e.to_string())?;
    let config = service.load_app_config().map_err(|e| e.to_string())?;
    Ok(config.state.last_project)
}

#[tauri::command]
pub fn config_set_last_project(project_path: Option<String>) -> Result<(), String> {
    let service = ConfigService::new().map_err(|e| e.to_string())?;
    let mut config = service.load_app_config().map_err(|e| e.to_string())?;
    config.state.last_project = project_path;
    service.save_app_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_project_exists(project_path: String) -> bool {
    ConfigService::project_config_exists(Path::new(&project_path))
}

#[tauri::command]
pub fn config_load_project(project_path: String) -> Result<ProjectConfig, String> {
    ConfigService::load_project_config(Path::new(&project_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_get_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            models: vec![
                "claude-sonnet-4-20250514".to_string(),
                "claude-opus-4-20250514".to_string(),
                "claude-3-5-sonnet-20241022".to_string(),
                "claude-3-5-haiku-20241022".to_string(),
            ],
            default_api_key_env: "ANTHROPIC_API_KEY".to_string(),
        },
        ProviderInfo {
            id: "gemini".to_string(),
            name: "Gemini".to_string(),
            models: vec![
                "gemini-2.0-flash".to_string(),
                "gemini-2.0-flash-lite".to_string(),
                "gemini-1.5-pro".to_string(),
                "gemini-1.5-flash".to_string(),
            ],
            default_api_key_env: "GEMINI_API_KEY".to_string(),
        },
    ]
}

#[tauri::command]
pub fn config_save_project(
    app_handle: AppHandle,
    project_path: String,
    config: ProjectConfig,
) -> Result<(), String> {
    ConfigService::save_project_config(Path::new(&project_path), &config)
        .map_err(|e| e.to_string())?;

    // Emit event for listeners (agent marks itself stale, frontend refreshes)
    let _ = app_handle.emit(
        "config-changed",
        ConfigChangedPayload {
            project_path: project_path.clone(),
        },
    );

    Ok(())
}

// Tool Descriptions (read-only, returns embedded defaults)

#[tauri::command]
pub fn config_get_tool_descriptions() -> HashMap<String, String> {
    get_tool_descriptions()
}

// Default System Prompt Command (read-only, returns embedded default)

#[tauri::command]
pub fn config_get_default_system_prompt() -> String {
    DEFAULT_SYSTEM_PROMPT.to_string()
}

// AGENTS.md Commands

#[tauri::command]
pub fn config_load_agents_md(project_path: String) -> Result<Option<String>, String> {
    ConfigService::load_agents_md(Path::new(&project_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_save_agents_md(
    app_handle: AppHandle,
    project_path: String,
    content: Option<String>,
) -> Result<(), String> {
    ConfigService::save_agents_md(Path::new(&project_path), content).map_err(|e| e.to_string())?;

    // Emit event for agent to reload memory
    let _ = app_handle.emit(
        "config-changed",
        ConfigChangedPayload {
            project_path: project_path.clone(),
        },
    );

    Ok(())
}
