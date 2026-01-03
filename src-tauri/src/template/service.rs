use std::fs;
use std::path::PathBuf;

use directories::ProjectDirs;
use handlebars::Handlebars;

use super::defaults::{DEFAULT_COMMIT_TEMPLATE, DEFAULT_REVIEW_COMMENTS_TEMPLATE};
use super::error::TemplateError;
use super::types::{CommitContext, ReviewCommentsContext, TemplateContent};

const TEMPLATES_DIR: &str = "templates";
const REVIEW_COMMENTS_FILENAME: &str = "review-comments.md";
const COMMIT_FILENAME: &str = "commit.md";

pub struct TemplateService {
    config_dir: PathBuf,
}

impl TemplateService {
    pub fn new() -> Result<Self, TemplateError> {
        let project_dirs =
            ProjectDirs::from("", "", "devflow").ok_or(TemplateError::NoConfigDir)?;

        let config_dir = project_dirs.config_dir().to_path_buf();

        Ok(Self { config_dir })
    }

    #[cfg(test)]
    pub fn with_config_dir(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    fn templates_dir(&self) -> PathBuf {
        self.config_dir.join(TEMPLATES_DIR)
    }

    fn review_comments_path(&self) -> PathBuf {
        self.templates_dir().join(REVIEW_COMMENTS_FILENAME)
    }

    fn commit_path(&self) -> PathBuf {
        self.templates_dir().join(COMMIT_FILENAME)
    }

    fn ensure_templates_dir(&self) -> Result<(), TemplateError> {
        let dir = self.templates_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| TemplateError::CreateDirError {
                path: dir,
                source: e,
            })?;
        }
        Ok(())
    }

    pub fn load_review_comments_template(&self) -> Result<String, TemplateError> {
        let path = self.review_comments_path();

        if path.exists() {
            fs::read_to_string(&path).map_err(|e| TemplateError::ReadError { path, source: e })
        } else {
            Ok(DEFAULT_REVIEW_COMMENTS_TEMPLATE.to_string())
        }
    }

    pub fn load_commit_template(&self) -> Result<String, TemplateError> {
        let path = self.commit_path();

        if path.exists() {
            fs::read_to_string(&path).map_err(|e| TemplateError::ReadError { path, source: e })
        } else {
            Ok(DEFAULT_COMMIT_TEMPLATE.to_string())
        }
    }

    pub fn load_templates(&self) -> Result<TemplateContent, TemplateError> {
        Ok(TemplateContent {
            review_comments: self.load_review_comments_template()?,
            commit: self.load_commit_template()?,
        })
    }

    pub fn save_review_comments_template(&self, content: &str) -> Result<(), TemplateError> {
        self.ensure_templates_dir()?;
        let path = self.review_comments_path();
        fs::write(&path, content).map_err(|e| TemplateError::WriteError { path, source: e })
    }

    pub fn save_commit_template(&self, content: &str) -> Result<(), TemplateError> {
        self.ensure_templates_dir()?;
        let path = self.commit_path();
        fs::write(&path, content).map_err(|e| TemplateError::WriteError { path, source: e })
    }

    pub fn save_templates(&self, templates: &TemplateContent) -> Result<(), TemplateError> {
        self.save_review_comments_template(&templates.review_comments)?;
        self.save_commit_template(&templates.commit)?;
        Ok(())
    }

    pub fn render_review_comments(
        &self,
        context: &ReviewCommentsContext,
    ) -> Result<String, TemplateError> {
        let template = self.load_review_comments_template()?;
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);
        handlebars.register_template_string("review_comments", &template)?;
        let result = handlebars.render("review_comments", context)?;
        Ok(result)
    }

    pub fn render_commit(&self, context: &CommitContext) -> Result<String, TemplateError> {
        let template = self.load_commit_template()?;
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false);
        handlebars.register_template_string("commit", &template)?;
        let result = handlebars.render("commit", context)?;
        Ok(result)
    }

    pub fn get_defaults() -> TemplateContent {
        TemplateContent {
            review_comments: DEFAULT_REVIEW_COMMENTS_TEMPLATE.to_string(),
            commit: DEFAULT_COMMIT_TEMPLATE.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_temp_service() -> (TemplateService, TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let service = TemplateService::with_config_dir(temp_dir.path().to_path_buf());
        (service, temp_dir)
    }

    #[test]
    fn test_load_default_templates() {
        let (service, _temp_dir) = create_temp_service();

        let review = service.load_review_comments_template().unwrap();
        assert!(review.contains("{{global_comment}}"));

        let commit = service.load_commit_template().unwrap();
        assert!(commit.contains("{{instructions}}"));
    }

    #[test]
    fn test_save_and_load_templates() {
        let (service, _temp_dir) = create_temp_service();

        let custom_review = "Custom review: {{global_comment}}";
        let custom_commit = "Custom commit: {{instructions}}";

        service
            .save_review_comments_template(custom_review)
            .unwrap();
        service.save_commit_template(custom_commit).unwrap();

        assert_eq!(
            service.load_review_comments_template().unwrap(),
            custom_review
        );
        assert_eq!(service.load_commit_template().unwrap(), custom_commit);
    }

    #[test]
    fn test_render_review_comments() {
        let (service, _temp_dir) = create_temp_service();

        let context = ReviewCommentsContext {
            comments: vec![super::super::types::ReviewComment {
                file: "src/main.rs".to_string(),
                lines: super::super::types::LineRange { start: 10, end: 15 },
                selected_code: "fn main() {}".to_string(),
                text: "Consider adding error handling".to_string(),
            }],
            global_comment: "Good progress!".to_string(),
        };

        let result = service.render_review_comments(&context).unwrap();
        assert!(result.contains("Good progress!"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("Consider adding error handling"));
    }

    #[test]
    fn test_render_commit() {
        let (service, _temp_dir) = create_temp_service();

        let context = CommitContext {
            instructions: "Fix the login bug".to_string(),
        };

        let result = service.render_commit(&context).unwrap();
        assert!(result.contains("Fix the login bug"));
    }

    #[test]
    fn test_get_defaults() {
        let defaults = TemplateService::get_defaults();
        assert!(defaults.review_comments.contains("{{global_comment}}"));
        assert!(defaults.commit.contains("{{instructions}}"));
    }
}
