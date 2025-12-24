use std::path::Path;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::error::AgentError;
use super::provider::ProviderAdapter;
use super::providers::create_provider_adapter;

pub struct AgentState {
    pub adapter: Option<Arc<dyn ProviderAdapter>>,
    pub project_path: Option<String>,
    pub cancel_token: Option<CancellationToken>,
    pub is_running: bool,
    pub config_stale: bool,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            adapter: None,
            project_path: None,
            cancel_token: None,
            is_running: false,
            config_stale: false,
        }
    }

    pub fn mark_config_stale(&mut self) {
        self.config_stale = true;
    }

    pub fn needs_reload(&self, project_path: &str) -> bool {
        self.config_stale
            || self.adapter.is_none()
            || self.project_path.as_deref() != Some(project_path)
    }

    pub fn initialize(&mut self, project_path: &str) -> Result<(), AgentError> {
        let path = Path::new(project_path);
        let adapter = create_provider_adapter(path)?;
        self.adapter = Some(adapter);
        self.project_path = Some(project_path.to_string());
        self.config_stale = false;
        Ok(())
    }

    pub fn get_adapter(&self) -> Option<Arc<dyn ProviderAdapter>> {
        self.adapter.clone()
    }

    pub fn start_run(&mut self) -> CancellationToken {
        let token = CancellationToken::new();
        self.cancel_token = Some(token.clone());
        self.is_running = true;
        token
    }

    pub fn cancel(&mut self) {
        if let Some(token) = self.cancel_token.take() {
            token.cancel();
        }
        self.is_running = false;
    }

    pub fn finish_run(&mut self) {
        self.cancel_token = None;
        self.is_running = false;
    }

    pub fn clear(&mut self) {
        self.cancel();
        self.adapter = None;
        self.project_path = None;
        self.config_stale = false;
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_needs_reload() {
        let state = AgentState::new();
        assert!(state.needs_reload("/some/path"));
    }

    #[test]
    fn test_needs_reload_when_stale() {
        let mut state = AgentState::new();
        state.project_path = Some("/some/path".to_string());
        state.config_stale = true;
        // Even with matching path, stale flag triggers reload
        assert!(state.needs_reload("/some/path"));
    }

    #[test]
    fn test_needs_reload_when_path_differs() {
        let mut state = AgentState::new();
        state.project_path = Some("/some/path".to_string());
        state.config_stale = false;
        assert!(state.needs_reload("/different/path"));
    }

    #[test]
    fn test_mark_config_stale() {
        let mut state = AgentState::new();
        assert!(!state.config_stale);
        state.mark_config_stale();
        assert!(state.config_stale);
    }

    #[test]
    fn test_clear_resets_stale() {
        let mut state = AgentState::new();
        state.config_stale = true;
        state.project_path = Some("/path".to_string());
        state.clear();
        assert!(!state.config_stale);
        assert!(state.project_path.is_none());
    }
}
