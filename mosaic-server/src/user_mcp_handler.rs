//! User-based MCP handler
//!
//! Maintains one MCP server instance per authenticated user,
//! allowing stateful sessions across multiple HTTP requests.

use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use mosaic_mcp::Mosaic;
use mosaic_serve::Serve;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info};

/// User-specific MCP server instance
struct UserMCPServer {
    mosaic: Mosaic,
    last_accessed: std::time::Instant,
}

/// Global storage for user MCP servers
pub struct UserMCPHandler {
    servers: Arc<RwLock<HashMap<String, Arc<Mutex<UserMCPServer>>>>>,
    serve: Arc<Mutex<Serve>>,
}

impl UserMCPHandler {
    pub fn new(serve: Arc<Mutex<Serve>>) -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            serve,
        }
    }

    /// Get or create MCP server for a user
    async fn get_or_create_server(&self, user_id: &str) -> Arc<Mutex<UserMCPServer>> {
        let mut servers = self.servers.write().await;

        if let Some(server) = servers.get(user_id) {
            // Update last accessed time
            let mut srv = server.lock().await;
            srv.last_accessed = std::time::Instant::now();
            drop(srv);
            info!("♻️  Reusing MCP server for user: {}", user_id);
            return server.clone();
        }

        // Create new server for this user
        info!("📝 Creating new MCP server for user: {}", user_id);
        let mosaic = Mosaic::with_shared_serve(self.serve.clone());
        let server = Arc::new(Mutex::new(UserMCPServer {
            mosaic,
            last_accessed: std::time::Instant::now(),
        }));

        servers.insert(user_id.to_string(), server.clone());
        server
    }

    /// Clean up inactive servers (call periodically)
    pub async fn cleanup_inactive(&self, max_age: std::time::Duration) {
        let mut servers = self.servers.write().await;
        let now = std::time::Instant::now();

        servers.retain(|user_id, server| {
            let srv = server.try_lock();
            if let Ok(srv) = srv {
                let age = now.duration_since(srv.last_accessed);
                if age > max_age {
                    info!("🗑️  Removing inactive MCP server for user: {}", user_id);
                    return false;
                }
            }
            true
        });
    }

    /// Handle MCP request for a user
    pub async fn handle_request(&self, request: Request<Body>) -> Response<Body> {
        // Extract user_id from request extensions (set by OAuth middleware)
        let user_id = match request.extensions().get::<String>() {
            Some(id) => id.clone(),
            None => {
                error!("No user_id found in request extensions");
                return Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Body::from("User ID not found"))
                    .unwrap();
            }
        };

        debug!("Handling MCP request for user: {}", user_id);

        // Get or create MCP server for this user
        let server = self.get_or_create_server(&user_id).await;
        let mut srv = server.lock().await;

        // Forward request to the user's MCP server
        // TODO: Implement proper request forwarding to Mosaic instance
        // For now, return a placeholder
        drop(srv);

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#))
            .unwrap()
    }
}
