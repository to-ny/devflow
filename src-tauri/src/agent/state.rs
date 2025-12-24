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
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            adapter: None,
            project_path: None,
            cancel_token: None,
            is_running: false,
        }
    }

    pub fn initialize(&mut self, project_path: &str) -> Result<(), AgentError> {
        let path = Path::new(project_path);
        let adapter = create_provider_adapter(path)?;
        self.adapter = Some(adapter);
        self.project_path = Some(project_path.to_string());
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
    }
}

impl Default for AgentState {
    fn default() -> Self {
        Self::new()
    }
}
