//! User-based MCP session manager
//!
//! Unlike LocalSessionManager which creates sessions per-connection,
//! this manager maintains one session per authenticated user ID.
//! This allows stateless HTTP requests from the same user to share a session.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Session manager that uses user IDs as session keys
#[derive(Clone)]
pub struct UserSessionManager {
    sessions: Arc<RwLock<HashMap<String, ()>>>,
}

impl UserSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a session for a user ID
    pub async fn get_or_create(&self, user_id: &str) -> String {
        let mut sessions = self.sessions.write().await;

        if !sessions.contains_key(user_id) {
            tracing::info!("📝 Creating new MCP session for user: {}", user_id);
            sessions.insert(user_id.to_string(), ());
        }

        user_id.to_string()
    }

    /// Remove a session
    pub async fn remove(&self, user_id: &str) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(user_id);
        tracing::info!("🗑️  Removed MCP session for user: {}", user_id);
    }
}

impl Default for UserSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
