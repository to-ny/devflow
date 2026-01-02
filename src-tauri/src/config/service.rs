use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use super::error::ConfigError;
use super::types::{AppConfig, ProjectConfig};

const APP_CONFIG_FILENAME: &str = "app.toml";
const PROJECT_CONFIG_DIR: &str = ".devflow";
const PROJECT_CONFIG_FILENAME: &str = "config.toml";
const AGENTS_MD_FILENAME: &str = "AGENTS.md";

pub struct ConfigService {
    app_data_dir: PathBuf,
}

impl ConfigService {
    pub fn new() -> Result<Self, ConfigError> {
        let project_dirs = ProjectDirs::from("", "", "devflow").ok_or(ConfigError::NoAppDataDir)?;

        let app_data_dir = project_dirs.data_dir().to_path_buf();

        Ok(Self { app_data_dir })
    }

    #[cfg(test)]
    pub fn with_app_data_dir(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    fn app_config_path(&self) -> PathBuf {
        self.app_data_dir.join(APP_CONFIG_FILENAME)
    }

    fn project_config_path(project_path: &Path) -> PathBuf {
        project_path
            .join(PROJECT_CONFIG_DIR)
            .join(PROJECT_CONFIG_FILENAME)
    }

    // App Config Methods

    pub fn load_app_config(&self) -> Result<AppConfig, ConfigError> {
        let path = self.app_config_path();

        if !path.exists() {
            return Ok(AppConfig::default());
        }

        let content = fs::read_to_string(&path).map_err(|e| ConfigError::ReadError {
            path: path.clone(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError { path, source: e })
    }

    pub fn save_app_config(&self, config: &AppConfig) -> Result<(), ConfigError> {
        let path = self.app_config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::CreateDirError {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let content = toml::to_string_pretty(config)?;
        fs::write(&path, content).map_err(|e| ConfigError::WriteError { path, source: e })
    }

    // Project Config Methods

    pub fn load_project_config(project_path: &Path) -> Result<ProjectConfig, ConfigError> {
        let path = Self::project_config_path(project_path);

        if !path.exists() {
            return Err(ConfigError::NotFound(path));
        }

        let content = fs::read_to_string(&path).map_err(|e| ConfigError::ReadError {
            path: path.clone(),
            source: e,
        })?;

        toml::from_str(&content).map_err(|e| ConfigError::ParseError { path, source: e })
    }

    pub fn save_project_config(
        project_path: &Path,
        config: &ProjectConfig,
    ) -> Result<(), ConfigError> {
        let path = Self::project_config_path(project_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::CreateDirError {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        let content = toml::to_string_pretty(config)?;
        fs::write(&path, content).map_err(|e| ConfigError::WriteError { path, source: e })
    }

    pub fn project_config_exists(project_path: &Path) -> bool {
        Self::project_config_path(project_path).exists()
    }

    // AGENTS.md Methods

    fn agents_md_path(project_path: &Path) -> PathBuf {
        project_path.join(AGENTS_MD_FILENAME)
    }

    /// Load AGENTS.md (None if missing)
    pub fn load_agents_md(project_path: &Path) -> Result<Option<String>, ConfigError> {
        let path = Self::agents_md_path(project_path);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).map_err(|e| ConfigError::ReadError {
            path: path.clone(),
            source: e,
        })?;

        Ok(Some(content))
    }

    /// Save AGENTS.md (skips empty content)
    pub fn save_agents_md(project_path: &Path, content: Option<String>) -> Result<(), ConfigError> {
        let path = Self::agents_md_path(project_path);

        match content {
            Some(c) if !c.trim().is_empty() => {
                fs::write(&path, c).map_err(|e| ConfigError::WriteError { path, source: e })
            }
            _ => Ok(()),
        }
    }

    /// Check if AGENTS.md exists
    pub fn agents_md_exists(project_path: &Path) -> bool {
        Self::agents_md_path(project_path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_dir() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn test_project_config() -> ProjectConfig {
        ProjectConfig {
            agent: AgentConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                max_tokens: 8192,
                context_limit: None,
            },
            prompts: PromptsConfig::default(),
            execution: ExecutionConfig {
                timeout_secs: 30,
                max_tool_iterations: 50,
                max_agent_depth: 3,
            },
            notifications: NotificationsConfig::default(),
            search: SearchConfig::default(),
            system_prompt: None,
            tool_descriptions: None,
            extraction_prompt: None,
            agent_prompts: None,
        }
    }

    #[test]
    fn test_load_app_config_default_when_missing() {
        let temp_dir = create_temp_dir();
        let service = ConfigService::with_app_data_dir(temp_dir.path().to_path_buf());

        let config = service.load_app_config().unwrap();
        assert!(config.state.last_project.is_none());
    }

    #[test]
    fn test_save_and_load_app_config() {
        let temp_dir = create_temp_dir();
        let service = ConfigService::with_app_data_dir(temp_dir.path().to_path_buf());

        let config = AppConfig {
            state: AppState {
                last_project: Some("/path/to/project".to_string()),
            },
        };

        service.save_app_config(&config).unwrap();
        let loaded = service.load_app_config().unwrap();

        assert_eq!(
            loaded.state.last_project,
            Some("/path/to/project".to_string())
        );
    }

    #[test]
    fn test_load_project_config_not_found() {
        let temp_dir = create_temp_dir();
        let result = ConfigService::load_project_config(temp_dir.path());

        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_save_and_load_project_config() {
        let temp_dir = create_temp_dir();

        let config = ProjectConfig {
            agent: AgentConfig {
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4-20250514".to_string(),
                api_key_env: "ANTHROPIC_API_KEY".to_string(),
                max_tokens: 8192,
                context_limit: None,
            },
            prompts: PromptsConfig {
                pre: "You are a helpful assistant.".to_string(),
                post: "Be concise.".to_string(),
            },
            execution: ExecutionConfig {
                timeout_secs: 60,
                max_tool_iterations: 50,
                max_agent_depth: 3,
            },
            notifications: NotificationsConfig {
                on_complete: vec![NotificationAction::Window],
                on_error: vec![NotificationAction::Sound, NotificationAction::Window],
            },
            search: SearchConfig::default(),
            system_prompt: None,
            tool_descriptions: None,
            extraction_prompt: None,
            agent_prompts: None,
        };

        ConfigService::save_project_config(temp_dir.path(), &config).unwrap();
        let loaded = ConfigService::load_project_config(temp_dir.path()).unwrap();

        assert_eq!(loaded.agent.provider, "anthropic");
        assert_eq!(loaded.agent.model, "claude-sonnet-4-20250514");
        assert_eq!(loaded.execution.timeout_secs, 60);
        assert_eq!(loaded.notifications.on_error.len(), 2);
    }

    #[test]
    fn test_project_config_with_defaults() {
        let temp_dir = create_temp_dir();
        let config_dir = temp_dir.path().join(".devflow");
        fs::create_dir_all(&config_dir).unwrap();

        let minimal_config = r#"
[agent]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096

[execution]
timeout_secs = 120
max_tool_iterations = 50
"#;

        fs::write(config_dir.join("config.toml"), minimal_config).unwrap();

        let loaded = ConfigService::load_project_config(temp_dir.path()).unwrap();

        assert_eq!(loaded.agent.provider, "anthropic");
        assert_eq!(loaded.execution.timeout_secs, 120);
        assert_eq!(loaded.execution.max_tool_iterations, 50);
        assert!(loaded.prompts.pre.is_empty());
    }

    #[test]
    fn test_parse_error_on_malformed_config() {
        let temp_dir = create_temp_dir();
        let config_dir = temp_dir.path().join(".devflow");
        fs::create_dir_all(&config_dir).unwrap();

        fs::write(config_dir.join("config.toml"), "invalid toml {{{").unwrap();

        let result = ConfigService::load_project_config(temp_dir.path());
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));
    }

    #[test]
    fn test_parse_error_on_missing_required_fields() {
        let temp_dir = create_temp_dir();
        let config_dir = temp_dir.path().join(".devflow");
        fs::create_dir_all(&config_dir).unwrap();

        let incomplete_config = r#"
[agent]
provider = "anthropic"
"#;

        fs::write(config_dir.join("config.toml"), incomplete_config).unwrap();

        let result = ConfigService::load_project_config(temp_dir.path());
        assert!(matches!(result, Err(ConfigError::ParseError { .. })));
    }

    #[test]
    fn test_project_config_exists() {
        let temp_dir = create_temp_dir();

        assert!(!ConfigService::project_config_exists(temp_dir.path()));

        let config_dir = temp_dir.path().join(".devflow");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("config.toml"), "").unwrap();

        assert!(ConfigService::project_config_exists(temp_dir.path()));
    }

    #[test]
    fn test_save_creates_devflow_directory() {
        let temp_dir = create_temp_dir();

        let config = test_project_config();

        let devflow_dir = temp_dir.path().join(".devflow");
        assert!(!devflow_dir.exists());

        ConfigService::save_project_config(temp_dir.path(), &config).unwrap();
        assert!(devflow_dir.exists());
    }

    #[test]
    fn test_overwrite_existing_config() {
        let temp_dir = create_temp_dir();

        let config1 = test_project_config();
        ConfigService::save_project_config(temp_dir.path(), &config1).unwrap();

        let config2 = ProjectConfig {
            agent: AgentConfig {
                provider: "gemini".to_string(),
                model: "gemini-2.0-flash".to_string(),
                api_key_env: "GEMINI_API_KEY".to_string(),
                max_tokens: 4096,
                context_limit: None,
            },
            execution: ExecutionConfig {
                timeout_secs: 60,
                max_tool_iterations: 100,
                max_agent_depth: 5,
            },
            ..test_project_config()
        };

        ConfigService::save_project_config(temp_dir.path(), &config2).unwrap();

        let loaded = ConfigService::load_project_config(temp_dir.path()).unwrap();
        assert_eq!(loaded.agent.provider, "gemini");
        assert_eq!(loaded.agent.model, "gemini-2.0-flash");
        assert_eq!(loaded.execution.timeout_secs, 60);
    }

    #[test]
    fn test_app_config_update_last_project() {
        let temp_dir = create_temp_dir();
        let service = ConfigService::with_app_data_dir(temp_dir.path().to_path_buf());

        let config = AppConfig {
            state: AppState {
                last_project: Some("/first/project".to_string()),
            },
        };
        service.save_app_config(&config).unwrap();

        let updated_config = AppConfig {
            state: AppState {
                last_project: Some("/second/project".to_string()),
            },
        };
        service.save_app_config(&updated_config).unwrap();

        let loaded = service.load_app_config().unwrap();
        assert_eq!(
            loaded.state.last_project,
            Some("/second/project".to_string())
        );
    }

    #[test]
    fn test_app_config_clear_last_project() {
        let temp_dir = create_temp_dir();
        let service = ConfigService::with_app_data_dir(temp_dir.path().to_path_buf());

        let config = AppConfig {
            state: AppState {
                last_project: Some("/some/project".to_string()),
            },
        };
        service.save_app_config(&config).unwrap();

        let cleared_config = AppConfig {
            state: AppState { last_project: None },
        };
        service.save_app_config(&cleared_config).unwrap();

        let loaded = service.load_app_config().unwrap();
        assert!(loaded.state.last_project.is_none());
    }

    #[test]
    fn test_notification_actions_serialization() {
        let temp_dir = create_temp_dir();

        let config = ProjectConfig {
            notifications: NotificationsConfig {
                on_complete: vec![NotificationAction::Sound, NotificationAction::Window],
                on_error: vec![NotificationAction::Window],
            },
            ..test_project_config()
        };

        ConfigService::save_project_config(temp_dir.path(), &config).unwrap();
        let loaded = ConfigService::load_project_config(temp_dir.path()).unwrap();

        assert_eq!(loaded.notifications.on_complete.len(), 2);
        assert!(loaded
            .notifications
            .on_complete
            .contains(&NotificationAction::Sound));
        assert!(loaded
            .notifications
            .on_complete
            .contains(&NotificationAction::Window));
        assert_eq!(loaded.notifications.on_error.len(), 1);
        assert!(loaded
            .notifications
            .on_error
            .contains(&NotificationAction::Window));
    }

    #[test]
    fn test_search_config_defaults() {
        let temp_dir = create_temp_dir();
        let config_dir = temp_dir.path().join(".devflow");
        fs::create_dir_all(&config_dir).unwrap();

        let config_without_search = r#"
[agent]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096

[execution]
timeout_secs = 30
max_tool_iterations = 50
"#;

        fs::write(config_dir.join("config.toml"), config_without_search).unwrap();

        let loaded = ConfigService::load_project_config(temp_dir.path()).unwrap();
        assert_eq!(loaded.search.provider, "duckduckgo");
        assert_eq!(loaded.search.max_results, 10);
    }

    #[test]
    fn test_max_agent_depth_default() {
        let temp_dir = create_temp_dir();
        let config_dir = temp_dir.path().join(".devflow");
        fs::create_dir_all(&config_dir).unwrap();

        let config_without_depth = r#"
[agent]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096

[execution]
timeout_secs = 30
max_tool_iterations = 50
"#;

        fs::write(config_dir.join("config.toml"), config_without_depth).unwrap();

        let loaded = ConfigService::load_project_config(temp_dir.path()).unwrap();
        assert_eq!(loaded.execution.max_agent_depth, 3);
    }

    #[test]
    fn test_config_toml_format() {
        let temp_dir = create_temp_dir();

        let config = ProjectConfig {
            prompts: PromptsConfig {
                pre: "System prompt".to_string(),
                post: "".to_string(),
            },
            ..test_project_config()
        };

        ConfigService::save_project_config(temp_dir.path(), &config).unwrap();

        let config_path = temp_dir.path().join(".devflow").join("config.toml");
        let content = fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[agent]"));
        assert!(content.contains("provider = \"anthropic\""));
        assert!(content.contains("[execution]"));
        assert!(content.contains("timeout_secs = 30"));
    }

    #[test]
    fn test_agents_md_load_not_found() {
        let temp_dir = create_temp_dir();
        let result = ConfigService::load_agents_md(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_agents_md_save_and_load() {
        let temp_dir = create_temp_dir();
        let content = "# Project Memory\n\nThis is test content.".to_string();

        ConfigService::save_agents_md(temp_dir.path(), Some(content.clone())).unwrap();

        let loaded = ConfigService::load_agents_md(temp_dir.path()).unwrap();
        assert_eq!(loaded, Some(content));
    }

    #[test]
    fn test_agents_md_not_saved_when_empty() {
        let temp_dir = create_temp_dir();

        ConfigService::save_agents_md(temp_dir.path(), Some("  ".to_string())).unwrap();

        assert!(!ConfigService::agents_md_exists(temp_dir.path()));
    }

    #[test]
    fn test_agents_md_exists() {
        let temp_dir = create_temp_dir();

        assert!(!ConfigService::agents_md_exists(temp_dir.path()));

        fs::write(temp_dir.path().join("AGENTS.md"), "content").unwrap();

        assert!(ConfigService::agents_md_exists(temp_dir.path()));
    }
}
