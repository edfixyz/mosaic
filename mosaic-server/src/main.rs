use anyhow::Result;
use axum::{
    Router,
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use clap::Parser;
use mosaic_fi::note::MosaicNote;
use mosaic_mcp::Mosaic;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use mosaic_serve::{get_notes, post_note};

#[derive(Parser, Debug)]
#[command(name = "mosaic-server")]
#[command(about = "Mosaic server with MCP and REST API endpoints", long_about = None)]
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
}

// Request/Response types for HTTP API
#[derive(Debug, Deserialize)]
struct PostNoteRequest {
    note: MosaicNote,
}

#[derive(Debug, Serialize)]
struct GetNotesResponse {
    notes: Vec<MosaicNote>,
}

// POST /market/:market
async fn post_note_handler(
    Path(market): Path<String>,
    Json(payload): Json<PostNoteRequest>,
) -> impl IntoResponse {
    // Validate market is a valid UUID
    if Uuid::parse_str(&market).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid market UUID"})),
        )
            .into_response();
    }

    // Post the note
    if let Err(_) = post_note(market, payload.note).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to post note"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"message": "Note posted successfully"})),
    )
        .into_response()
}

// GET /market/:market
async fn get_notes_handler(Path(market): Path<String>) -> impl IntoResponse {
    // Validate market is a valid UUID
    if Uuid::parse_str(&market).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid market UUID"})),
        )
            .into_response();
    }

    // Get notes and handle errors
    let notes = match get_notes(market).await {
        Ok(notes) => notes,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to get notes"})),
            )
                .into_response();
        }
    };

    let response = GetNotesResponse { notes };

    (StatusCode::OK, Json(response)).into_response()
}

async fn run_mcp_server(port: u16) -> Result<()> {
    let bind_address = format!("127.0.0.1:{}", port);
    tracing::info!("Starting MCP server on {}", bind_address);

    let service = StreamableHttpService::new(
        || Ok(Mosaic::new()),
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

async fn run_rest_server(port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting REST API server on {}", addr);

    let app = Router::new()
        .route("/market/{market}", post(post_note_handler))
        .route("/market/{market}", get(get_notes_handler));

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}

async fn run_combined_server_same_port(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    tracing::info!("Starting combined MCP and REST API server on {}", addr);

    // Create MCP service
    let mcp_service = StreamableHttpService::new(
        || Ok(Mosaic::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create HTTP REST API routes
    let http_routes = Router::new()
        .route("/market/{market}", post(post_note_handler))
        .route("/market/{market}", get(get_notes_handler));

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

async fn run_both_servers_different_ports(mcp_port: u16, rest_port: u16) -> Result<()> {
    tracing::info!("Starting MCP server on 127.0.0.1:{}", mcp_port);
    tracing::info!("Starting REST API server on 0.0.0.0:{}", rest_port);

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
        tokio::spawn(async move {
            let bind_address = format!("127.0.0.1:{}", mcp_port);
            let service = StreamableHttpService::new(
                || Ok(Mosaic::new()),
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
        tokio::spawn(async move {
            let addr = SocketAddr::from(([0, 0, 0, 0], rest_port));
            let app = Router::new()
                .route("/market/{market}", post(post_note_handler))
                .route("/market/{market}", get(get_notes_handler));
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
async fn main() -> Result<()> {
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

    match (args.mcp, args.rest) {
        (true, false) => {
            // MCP server only
            run_mcp_server(args.mcp_port).await?;
        }
        (false, true) => {
            // REST API server only
            run_rest_server(args.rest_port).await?;
        }
        (true, true) => {
            // Both servers
            if args.mcp_port == args.rest_port {
                // Same port: use combined router for efficiency
                run_combined_server_same_port(args.mcp_port).await?;
            } else {
                // Different ports: run two separate servers concurrently
                run_both_servers_different_ports(args.mcp_port, args.rest_port).await?;
            }
        }
        (false, false) => {
            unreachable!("Already validated that at least one server is enabled");
        }
    }

    Ok(())
}
