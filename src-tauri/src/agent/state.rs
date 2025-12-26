use std::path::Path;
use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use super::error::AgentError;
use super::provider::ProviderAdapter;
use super::providers::create_provider_adapter;
use super::tools::SessionState;

pub struct AgentState {
    pub adapter: Option<Arc<dyn ProviderAdapter>>,
    pub project_path: Option<String>,
    pub cancel_token: Option<CancellationToken>,
    pub is_running: bool,
    pub config_stale: bool,
    pub session: SessionState,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            adapter: None,
            project_path: None,
            cancel_token: None,
            is_running: false,
            config_stale: false,
            session: SessionState::new(),
        }
    }

    pub fn get_session(&self) -> SessionState {
        self.session.clone()
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
        self.session = SessionState::new();
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

    #[test]
    fn test_default_state_is_not_running() {
        let state = AgentState::new();
        assert!(!state.is_running);
    }

    #[test]
    fn test_start_run_sets_is_running() {
        let mut state = AgentState::new();
        assert!(!state.is_running);

        let _token = state.start_run();
        assert!(state.is_running);
    }

    #[test]
    fn test_start_run_returns_cancel_token() {
        let mut state = AgentState::new();
        let token = state.start_run();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_finish_run_clears_running_state() {
        let mut state = AgentState::new();
        let _token = state.start_run();
        assert!(state.is_running);

        state.finish_run();
        assert!(!state.is_running);
    }

    #[test]
    fn test_finish_run_clears_cancel_token() {
        let mut state = AgentState::new();
        let _token = state.start_run();
        assert!(state.cancel_token.is_some());

        state.finish_run();
        assert!(state.cancel_token.is_none());
    }

    #[test]
    fn test_cancel_sets_is_running_false() {
        let mut state = AgentState::new();
        let _token = state.start_run();
        assert!(state.is_running);

        state.cancel();
        assert!(!state.is_running);
    }

    #[test]
    fn test_cancel_triggers_cancellation_token() {
        let mut state = AgentState::new();
        let token = state.start_run();
        assert!(!token.is_cancelled());

        state.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancel_when_not_running_is_safe() {
        let mut state = AgentState::new();
        assert!(!state.is_running);
        state.cancel();
        assert!(!state.is_running);
    }

    #[test]
    fn test_get_adapter_returns_none_when_not_initialized() {
        let state = AgentState::new();
        assert!(state.get_adapter().is_none());
    }

    #[test]
    fn test_clear_removes_adapter() {
        let mut state = AgentState::new();
        state.project_path = Some("/path".to_string());
        state.clear();
        assert!(state.adapter.is_none());
    }

    #[test]
    fn test_clear_cancels_any_running_operation() {
        let mut state = AgentState::new();
        let token = state.start_run();
        assert!(state.is_running);

        state.clear();

        assert!(!state.is_running);
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_default_trait_creates_new_state() {
        let state = AgentState::default();
        assert!(!state.is_running);
        assert!(state.adapter.is_none());
        assert!(state.project_path.is_none());
    }

    #[test]
    fn test_multiple_start_run_calls_create_new_tokens() {
        let mut state = AgentState::new();

        let token1 = state.start_run();
        state.finish_run();

        let token2 = state.start_run();
        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        state.cancel();
        assert!(!token1.is_cancelled());
        assert!(token2.is_cancelled());
    }
}
