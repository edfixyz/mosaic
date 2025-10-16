use axum::{
    Router,
    extract::{Json, Path, State as AxumState},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use clap::Parser;
use mosaic_fi::note::MosaicNote;
use mosaic_mcp::Mosaic;
use mosaic_miden::Network;
use mosaic_serve::Serve;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "mosaic-server")]
#[command(about = "Mosaic server with MCP and REST endpoints", long_about = None)]
struct Args {
    /// Enable MCP server
    #[arg(long, default_value_t = false)]
    mcp: bool,

    /// MCP server port
    #[arg(long, default_value_t = 8000)]
    mcp_port: u16,

    /// Enable REST API server
    #[arg(long, default_value_t = false)]
    rest: bool,

    /// REST API server port
    #[arg(long, default_value_t = 3000)]
    rest_port: u16,

    /// Storage path for accounts
    #[arg(long, default_value = "./mosaic_store")]
    storage_path: String,
}

// Request/Response types for HTTP API
#[derive(Debug, Deserialize)]
struct DeskPushNoteRequest {
    note: MosaicNote,
}

#[derive(Debug, Serialize)]
struct DeskInfoResponse {
    desk_uuid: String,
    account_id: String,
    network: String,
    market: mosaic_fi::Market,
}

// GET /desk/{uuid}
async fn get_desk_info_handler(
    AxumState(serve): AxumState<Arc<Mutex<Serve>>>,
    Path(uuid_str): Path<String>,
) -> impl IntoResponse {
    // Parse UUID
    let desk_uuid = match uuid::Uuid::parse_str(&uuid_str) {
        Ok(uuid) => uuid,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid UUID: {}", e)})),
            )
                .into_response();
        }
    };

    // Get desk info
    let serve = serve.lock().await;
    match serve.get_desk_info(desk_uuid).await {
        Ok((account_id, network, market)) => {
            let response = DeskInfoResponse {
                desk_uuid: uuid_str,
                account_id,
                network: match network {
                    Network::Testnet => "Testnet".to_string(),
                    Network::Localnet => "Localnet".to_string(),
                },
                market,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to get desk info: {}", e)})),
        )
            .into_response(),
    }
}

// POST /desk/:uuid/note
async fn desk_push_note_handler(
    AxumState(serve): AxumState<Arc<Mutex<Serve>>>,
    Path(uuid_str): Path<String>,
    Json(payload): Json<DeskPushNoteRequest>,
) -> impl IntoResponse {
    // Parse UUID
    let desk_uuid = match uuid::Uuid::parse_str(&uuid_str) {
        Ok(uuid) => uuid,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid UUID: {}", e)})),
            )
                .into_response();
        }
    };

    // Push note to desk
    let serve = serve.lock().await;
    match serve.desk_push_note(desk_uuid, payload.note).await {
        Ok(note_id) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Note pushed to desk successfully",
                "desk_uuid": uuid_str,
                "note_id": note_id
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to push note: {}", e)})),
        )
            .into_response(),
    }
}

async fn run_mcp_server(port: u16, storage_path: String) -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = format!("127.0.0.1:{}", port);
    tracing::info!("Starting MCP server on {}", bind_address);
    tracing::info!("Using storage path: {}", storage_path);

    // Create shared Serve instance
    let mut serve = mosaic_serve::Serve::new(&storage_path)?;
    serve.init_desks().await?;
    let serve_state = Arc::new(Mutex::new(serve));

    let service = StreamableHttpService::new(
        move || Ok(Mosaic::with_shared_serve(serve_state.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = Router::new().nest_service("/mcp", service);
    let tcp_listener = tokio::net::TcpListener::bind(&bind_address).await?;

    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}

async fn run_rest_server(
    port: u16,
    storage_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting REST API server on {}", addr);
    tracing::info!("Using storage path: {}", storage_path);

    // Create shared Serve instance
    let mut serve = mosaic_serve::Serve::new(&storage_path)?;
    serve.init_desks().await?;
    let serve_state = Arc::new(Mutex::new(serve));

    let app = Router::new()
        .route("/desk/{uuid}", get(get_desk_info_handler))
        .route("/desk/{uuid}/note", post(desk_push_note_handler))
        .with_state(serve_state);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}

async fn run_combined_server_same_port(
    port: u16,
    storage_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port);
    tracing::info!("Starting combined MCP and REST API server on {}", addr);
    tracing::info!("Using storage path: {}", storage_path);

    // Create shared Serve instance
    let mut serve = mosaic_serve::Serve::new(&storage_path)?;
    serve.init_desks().await?;
    let serve_state = Arc::new(Mutex::new(serve));

    // Create MCP service with shared Serve instance
    let mcp_service = {
        let serve_clone = serve_state.clone();
        StreamableHttpService::new(
            move || Ok(Mosaic::with_shared_serve(serve_clone.clone())),
            LocalSessionManager::default().into(),
            Default::default(),
        )
    };

    // Create HTTP REST API routes with shared state
    let http_routes = Router::new()
        .route("/desk/{uuid}", get(get_desk_info_handler))
        .route("/desk/{uuid}/note", post(desk_push_note_handler))
        .with_state(serve_state);

    // Combine both into a single router
    let combined_router = Router::new()
        .nest_service("/mcp", mcp_service)
        .merge(http_routes);

    let tcp_listener = tokio::net::TcpListener::bind(&addr).await?;

    axum::serve(tcp_listener, combined_router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}

async fn run_both_servers_different_ports(
    mcp_port: u16,
    rest_port: u16,
    storage_path: String,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting MCP server on 127.0.0.1:{}", mcp_port);
    tracing::info!("Starting REST API server on 0.0.0.0:{}", rest_port);
    tracing::info!("Using storage path: {}", storage_path);

    // Create shared Serve instance for both servers
    let mut serve = mosaic_serve::Serve::new(&storage_path)?;
    serve.init_desks().await?;
    let serve_state = Arc::new(Mutex::new(serve));

    // Create a cancellation token for graceful shutdown
    let shutdown_token = tokio_util::sync::CancellationToken::new();
    let shutdown_token_clone = shutdown_token.clone();

    // Spawn ctrl-c handler
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        shutdown_token_clone.cancel();
    });

    // Run both servers concurrently
    let mcp_handle = {
        let shutdown = shutdown_token.clone();
        let serve_clone = serve_state.clone();
        tokio::spawn(async move {
            let bind_address = format!("127.0.0.1:{}", mcp_port);
            let service = StreamableHttpService::new(
                move || Ok(Mosaic::with_shared_serve(serve_clone.clone())),
                LocalSessionManager::default().into(),
                Default::default(),
            );
            let router = Router::new().nest_service("/mcp", service);
            let tcp_listener = tokio::net::TcpListener::bind(&bind_address).await?;

            axum::serve(tcp_listener, router)
                .with_graceful_shutdown(async move { shutdown.cancelled().await })
                .await
        })
    };

    let rest_handle = {
        let shutdown = shutdown_token.clone();
        let serve_clone = serve_state.clone();
        tokio::spawn(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], rest_port));

            let app = Router::new()
                .route("/desk/{uuid}", get(get_desk_info_handler))
                .route("/desk/{uuid}/note", post(desk_push_note_handler))
                .with_state(serve_clone);
            let listener = tokio::net::TcpListener::bind(addr).await?;

            axum::serve(listener, app)
                .with_graceful_shutdown(async move { shutdown.cancelled().await })
                .await
        })
    };

    // Wait for both servers to complete
    let (mcp_result, rest_result) = tokio::join!(mcp_handle, rest_handle);

    // Check for errors
    mcp_result??;
    rest_result??;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".to_string().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    // Validate that at least one server is enabled
    if !args.mcp && !args.rest {
        eprintln!("Error: At least one server must be enabled. Use --mcp or --rest or both.");
        eprintln!("\nUsage examples:");
        eprintln!(
            "  mosaic-server --mcp                      # Start MCP server only on port 8000"
        );
        eprintln!(
            "  mosaic-server --rest                     # Start REST API server only on port 3000"
        );
        eprintln!(
            "  mosaic-server --mcp --rest               # Start both servers (same port 8000)"
        );
        eprintln!("  mosaic-server --mcp --mcp-port 9000      # Start MCP server on custom port");
        eprintln!(
            "  mosaic-server --rest --rest-port 4000    # Start REST API server on custom port"
        );
        eprintln!(
            "  mosaic-server --mcp --rest --mcp-port 9000 --rest-port 9001  # Both on different ports"
        );
        std::process::exit(1);
    }

    // Create storage directory if it doesn't exist
    std::fs::create_dir_all(&args.storage_path)?;

    match (args.mcp, args.rest) {
        (true, false) => {
            // MCP server only
            run_mcp_server(args.mcp_port, args.storage_path).await?;
        }
        (false, true) => {
            // REST API server only
            run_rest_server(args.rest_port, args.storage_path).await?;
        }
        (true, true) => {
            // Both servers
            if args.mcp_port == args.rest_port {
                // Same port: use combined router for efficiency
                run_combined_server_same_port(args.mcp_port, args.storage_path).await?;
            } else {
                // Different ports: run two separate servers concurrently
                run_both_servers_different_ports(args.mcp_port, args.rest_port, args.storage_path)
                    .await?;
            }
        }
        (false, false) => {
            unreachable!("Already validated that at least one server is enabled");
        }
    }

    Ok(())
}
