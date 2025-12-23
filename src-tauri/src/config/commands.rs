use std::path::Path;

use super::service::ConfigService;
use super::types::ProjectConfig;

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
pub fn config_save_project(project_path: String, config: ProjectConfig) -> Result<(), String> {
    ConfigService::save_project_config(Path::new(&project_path), &config).map_err(|e| e.to_string())
}
