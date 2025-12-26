use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::tools::types::TodoItem;

#[derive(Clone)]
pub struct SessionState {
    todos: Arc<RwLock<Vec<TodoItem>>>,
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
        }
    }

    pub async fn get_todos(&self) -> Vec<TodoItem> {
        self.todos.read().await.clone()
    }

    pub async fn set_todos(&self, todos: Vec<TodoItem>) {
        *self.todos.write().await = todos;
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
}
