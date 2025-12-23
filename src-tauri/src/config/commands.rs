use super::service::ConfigService;

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
