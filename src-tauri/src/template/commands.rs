use super::service::TemplateService;
use super::types::{CommitContext, ReviewCommentsContext, TemplateContent};

#[tauri::command]
pub fn template_load() -> Result<TemplateContent, String> {
    let service = TemplateService::new().map_err(|e| e.to_string())?;
    service.load_templates().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn template_save(templates: TemplateContent) -> Result<(), String> {
    let service = TemplateService::new().map_err(|e| e.to_string())?;
    service
        .save_templates(&templates)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn template_render_review_comments(context: ReviewCommentsContext) -> Result<String, String> {
    let service = TemplateService::new().map_err(|e| e.to_string())?;
    service
        .render_review_comments(&context)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn template_render_commit(context: CommitContext) -> Result<String, String> {
    let service = TemplateService::new().map_err(|e| e.to_string())?;
    service.render_commit(&context).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn template_get_defaults() -> TemplateContent {
    TemplateService::get_defaults()
}
