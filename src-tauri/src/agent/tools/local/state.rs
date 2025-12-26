use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

use crate::agent::tools::types::TodoItem;

/// Result of plan approval
#[derive(Debug, Clone)]
pub enum PlanApproval {
    Approved,
    Rejected(Option<String>),
}

/// Inner state for plan approval channel
struct PlanApprovalState {
    plan: Option<String>,
    sender: Option<oneshot::Sender<PlanApproval>>,
    receiver: Option<oneshot::Receiver<PlanApproval>>,
}

#[derive(Clone)]
pub struct SessionState {
    todos: Arc<RwLock<Vec<TodoItem>>>,
    plan_approval: Arc<RwLock<PlanApprovalState>>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            todos: Arc::new(RwLock::new(Vec::new())),
            plan_approval: Arc::new(RwLock::new(PlanApprovalState {
                plan: None,
                sender: None,
                receiver: None,
            })),
        }
    }

    pub async fn get_todos(&self) -> Vec<TodoItem> {
        self.todos.read().await.clone()
    }

    pub async fn set_todos(&self, todos: Vec<TodoItem>) {
        *self.todos.write().await = todos;
    }

    /// Set a plan and create approval channel
    pub async fn set_plan(&self, plan: String) {
        let (tx, rx) = oneshot::channel();
        let mut state = self.plan_approval.write().await;
        state.plan = Some(plan);
        state.sender = Some(tx);
        state.receiver = Some(rx);
    }

    pub async fn get_plan(&self) -> Option<String> {
        self.plan_approval.read().await.plan.clone()
    }

    pub async fn clear_plan(&self) {
        let mut state = self.plan_approval.write().await;
        state.plan = None;
        state.sender = None;
        state.receiver = None;
    }

    /// Wait for plan approval from user
    /// Returns None if no plan is pending, otherwise waits for approval/rejection
    pub async fn wait_for_plan_approval(&self) -> Option<PlanApproval> {
        // Take the receiver out of the state
        let receiver = {
            let mut state = self.plan_approval.write().await;
            state.receiver.take()
        };

        match receiver {
            Some(rx) => {
                // Wait for approval signal
                match rx.await {
                    Ok(approval) => Some(approval),
                    Err(_) => Some(PlanApproval::Rejected(Some("Channel closed".to_string()))),
                }
            }
            None => None,
        }
    }

    /// Approve the pending plan (called by Tauri command)
    pub async fn approve_plan(&self) -> bool {
        let mut state = self.plan_approval.write().await;
        if let Some(sender) = state.sender.take() {
            state.plan = None;
            let _ = sender.send(PlanApproval::Approved);
            true
        } else {
            false
        }
    }

    /// Reject the pending plan (called by Tauri command)
    pub async fn reject_plan(&self, reason: Option<String>) -> bool {
        let mut state = self.plan_approval.write().await;
        if let Some(sender) = state.sender.take() {
            state.plan = None;
            let _ = sender.send(PlanApproval::Rejected(reason));
            true
        } else {
            false
        }
    }

    /// Check if a plan is pending approval
    pub async fn has_pending_plan(&self) -> bool {
        let state = self.plan_approval.read().await;
        state.plan.is_some() && state.sender.is_some()
    }

    #[cfg(test)]
    pub async fn todos_count(&self) -> usize {
        self.todos.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_state_todos() {
        let state = SessionState::new();
        assert!(state.get_todos().await.is_empty());

        let todos = vec![TodoItem {
            id: "1".to_string(),
            content: "Test".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
        }];

        state.set_todos(todos).await;
        assert_eq!(state.todos_count().await, 1);
    }

    #[tokio::test]
    async fn test_plan_approval() {
        let state = SessionState::new();

        // Set a plan
        state.set_plan("Test plan".to_string()).await;
        assert!(state.has_pending_plan().await);
        assert_eq!(state.get_plan().await, Some("Test plan".to_string()));

        // Clone for the approval task
        let state2 = state.clone();

        // Spawn approval in background
        let handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            state2.approve_plan().await
        });

        // Wait for approval
        let result = state.wait_for_plan_approval().await;
        assert!(matches!(result, Some(PlanApproval::Approved)));

        let approved = handle.await.unwrap();
        assert!(approved);
    }

    #[tokio::test]
    async fn test_plan_rejection() {
        let state = SessionState::new();

        state.set_plan("Test plan".to_string()).await;

        let state2 = state.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            state2.reject_plan(Some("Not good".to_string())).await
        });

        let result = state.wait_for_plan_approval().await;
        assert!(matches!(result, Some(PlanApproval::Rejected(Some(ref r))) if r == "Not good"));

        let rejected = handle.await.unwrap();
        assert!(rejected);
    }
}
