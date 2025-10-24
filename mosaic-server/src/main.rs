use axum::{
    Router,
    body::Body,
    extract::{Json, Path, State as AxumState},
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
use mosaic_fi::note::MosaicNote;
use mosaic_mcp::Mosaic;
use mosaic_miden::Network;
use mosaic_serve::{Serve, asset_store::default_assets};
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod helper;
mod oauth;

fn allowed_origins() -> AllowOrigin {
    match std::env::var("MOSAIC_CORS_ALLOWED_ORIGINS") {
        Ok(value) => {
            let origins: Vec<_> = value
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .filter_map(|origin| {
                    let normalized = origin.trim_end_matches('/');
                    match HeaderValue::from_str(normalized) {
                        Ok(value) => Some(value),
                        Err(err) => {
                            warn!(
                                "Ignoring invalid origin '{}' in MOSAIC_CORS_ALLOWED_ORIGINS: {}",
                                normalized, err
                            );
                            None
                        }
                    }
                })
                .collect();

            if origins.is_empty() {
                AllowOrigin::any()
            } else {
                AllowOrigin::list(origins)
            }
        }
        Err(_) => AllowOrigin::any(),
    }
}

fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(allowed_origins())
        .allow_methods(Any)
        .allow_headers(Any)
        .expose_headers([HeaderName::from_static("mcp-session-id")])
}

fn apply_desk_cors_headers(headers: &mut HeaderMap) {
    headers.insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-methods"),
        HeaderValue::from_static("POST, GET, OPTIONS"),
    );
    headers.insert(
        HeaderName::from_static("access-control-allow-headers"),
        HeaderValue::from_static("Content-Type"),
    );
    headers.insert(
        HeaderName::from_static("access-control-max-age"),
        HeaderValue::from_static("86400"),
    );
}

async fn desk_cors_middleware(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    apply_desk_cors_headers(response.headers_mut());
    response
}

async fn preflight_desk_handler() -> impl IntoResponse {
    let mut response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::empty())
        .unwrap();
    apply_desk_cors_headers(response.headers_mut());
    response
}

fn fallback_desk_url(account_id: &str) -> String {
    std::env::var("MOSAIC_SERVER")
        .map(|base| format!("{}/desk/{}", base.trim_end_matches('/'), account_id))
        .unwrap_or_else(|_| format!("/desk/{}", account_id))
}

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

    /// Disable OAuth authentication (for testing)
    #[arg(long, default_value_t = false)]
    no_auth: bool,
}

// Request/Response types for HTTP API
#[derive(Debug, Deserialize)]
struct DeskPushNoteRequest {
    note: MosaicNote,
}

#[derive(Debug, Serialize)]
struct DeskInfoResponse {
    desk_account: String,
    account_id: String,
    network: String,
    market: mosaic_fi::Market,
    base_account: String,
    quote_account: String,
    market_url: String,
    owner_account: String,
}

#[derive(Debug, Serialize)]
struct AssetSummary {
    account: String,
    symbol: String,
    #[serde(rename = "maxSupply")]
    max_supply: String,
    decimals: u8,
    verified: bool,
    owner: bool,
    hidden: bool,
}

// GET /desk/{account_id}
async fn get_desk_info_handler(
    AxumState(serve): AxumState<Arc<Mutex<Serve>>>,
    Path(account_id): Path<String>,
) -> impl IntoResponse {
    // Get desk info
    let serve = serve.lock().await;
    match serve.get_desk_info(&account_id).await {
        Ok((account_id, network, market)) => {
            let summary = serve.get_desk_market_summary(&account_id).ok().flatten();

            let (base_account, quote_account, market_url, owner_account) = summary
                .map(|s| {
                    (
                        s.base_account,
                        s.quote_account,
                        s.market_url,
                        s.owner_account,
                    )
                })
                .unwrap_or_else(|| {
                    (
                        String::new(),
                        String::new(),
                        fallback_desk_url(&account_id),
                        String::new(),
                    )
                });

            let response = DeskInfoResponse {
                desk_account: account_id.clone(),
                account_id,
                network: match network {
                    Network::Testnet => "Testnet".to_string(),
                    Network::Localnet => "Localnet".to_string(),
                },
                market,
                base_account,
                quote_account,
                market_url,
                owner_account,
            };
            let mut response = (StatusCode::OK, Json(response)).into_response();
            apply_desk_cors_headers(response.headers_mut());
            response
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to get desk info: {}", e)})),
        )
            .into_response(),
    }
}

// POST /desk/:account_id/note
async fn desk_push_note_handler(
    AxumState(serve): AxumState<Arc<Mutex<Serve>>>,
    Path(account_id): Path<String>,
    Json(payload): Json<DeskPushNoteRequest>,
) -> impl IntoResponse {
    // Push note to desk
    let serve = serve.lock().await;
    match serve.desk_push_note(&account_id, payload.note).await {
        Ok(note_id) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Note pushed to desk successfully",
                "desk_account": account_id,
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

async fn list_assets_handler() -> impl IntoResponse {
    let assets: Vec<AssetSummary> = default_assets()
        .into_iter()
        .map(|asset| AssetSummary {
            account: asset.account,
            symbol: asset.symbol,
            max_supply: asset.max_supply,
            decimals: asset.decimals,
            verified: asset.verified,
            owner: asset.owner,
            hidden: asset.hidden,
        })
        .collect();

    (StatusCode::OK, Json(assets))
}

async fn run_mcp_server(
    port: u16,
    storage_path: String,
    no_auth: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let bind_address = format!("127.0.0.1:{}", port);
    tracing::info!("Starting MCP server on {}", bind_address);
    tracing::info!("Using storage path: {}", storage_path);

    if no_auth {
        tracing::warn!("OAuth DISABLED - server is running without authentication!");
    } else {
        tracing::info!("OAuth enabled with Auth0 at auth.mosaic.edfi.xyz");
    }

    // Create shared Serve instance
    let mut serve = mosaic_serve::Serve::new(&storage_path)?;
    serve.init_desks().await?;
    let serve_state = Arc::new(Mutex::new(serve));

    // Create MCP service
    let serve_state_for_mcp = serve_state.clone();
    let mcp_service = StreamableHttpService::new(
        move || Ok(Mosaic::with_shared_serve(serve_state_for_mcp.clone())),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create CORS layer for all endpoints (MCP + OAuth)
    let cors = build_cors_layer();

    // Conditionally protect MCP endpoints with OAuth middleware + CORS
    let mcp_router = if no_auth {
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(cors.clone())
    } else {
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(middleware::from_fn(oauth::oauth_middleware))
            .layer(cors.clone())
    };

    // OAuth endpoints (publicly accessible)
    let bind_addr_for_metadata = bind_address.clone();
    let oauth_routes = Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(move || oauth::oauth_authorization_server_metadata(bind_addr_for_metadata.clone())),
        )
        .route("/oauth/register", post(oauth::oauth_register))
        .route("/oauth/token", post(oauth::oauth_token))
        .layer(cors.clone());

    // Public unauthenticated routes
    let desk_routes = Router::new()
        .route(
            "/desk/{account_id}",
            get(get_desk_info_handler).options(preflight_desk_handler),
        )
        .route(
            "/desk/{account_id}/note",
            post(desk_push_note_handler).options(preflight_desk_handler),
        )
        .layer(middleware::from_fn(desk_cors_middleware))
        .with_state(serve_state.clone());

    let asset_routes = Router::new()
        .route("/assets", get(list_assets_handler))
        .layer(cors);

    // Combine routes
    let router = Router::new()
        .merge(mcp_router)
        .merge(oauth_routes)
        .merge(asset_routes)
        .merge(desk_routes);

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

    let desk_routes = Router::new()
        .route(
            "/desk/{account_id}",
            get(get_desk_info_handler).options(preflight_desk_handler),
        )
        .route(
            "/desk/{account_id}/note",
            post(desk_push_note_handler).options(preflight_desk_handler),
        )
        .layer(middleware::from_fn(desk_cors_middleware))
        .with_state(serve_state.clone());

    let app = Router::new()
        .route("/assets", get(list_assets_handler))
        .merge(desk_routes);

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
    no_auth: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("127.0.0.1:{}", port);
    tracing::info!("Starting combined MCP and REST API server on {}", addr);
    tracing::info!("Using storage path: {}", storage_path);

    if no_auth {
        tracing::warn!("OAuth DISABLED - server is running without authentication!");
    } else {
        tracing::info!("OAuth enabled with Auth0 at auth.mosaic.edfi.xyz");
    }

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

    // Create CORS layer for all endpoints (MCP + OAuth)
    let cors = build_cors_layer();

    // Conditionally protect MCP endpoints with OAuth middleware + CORS
    let mcp_router = if no_auth {
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(cors.clone())
    } else {
        Router::new()
            .nest_service("/mcp", mcp_service)
            .layer(middleware::from_fn(oauth::oauth_middleware))
            .layer(cors.clone())
    };

    // OAuth endpoints (publicly accessible)
    let bind_addr_for_metadata = addr.clone();
    let oauth_routes = Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(move || oauth::oauth_authorization_server_metadata(bind_addr_for_metadata.clone())),
        )
        .route("/oauth/register", post(oauth::oauth_register))
        .route("/oauth/token", post(oauth::oauth_token))
        .layer(cors);

    let desk_routes = Router::new()
        .route(
            "/desk/{account_id}",
            get(get_desk_info_handler).options(preflight_desk_handler),
        )
        .route(
            "/desk/{account_id}/note",
            post(desk_push_note_handler).options(preflight_desk_handler),
        )
        .layer(middleware::from_fn(desk_cors_middleware))
        .with_state(serve_state.clone());

    let http_routes = Router::new().merge(desk_routes);

    let public_routes = Router::new()
        .route("/assets", get(list_assets_handler))
        .layer(build_cors_layer());

    // Combine all routes into a single router
    let combined_router = Router::new()
        .merge(mcp_router)
        .merge(oauth_routes)
        .merge(http_routes)
        .merge(public_routes);

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
    no_auth: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting MCP server on 127.0.0.1:{}", mcp_port);
    tracing::info!("Starting REST API server on 0.0.0.0:{}", rest_port);
    tracing::info!("Using storage path: {}", storage_path);

    if no_auth {
        tracing::warn!("OAuth DISABLED - MCP server is running without authentication!");
    } else {
        tracing::info!("OAuth enabled with Auth0 at auth.mosaic.edfi.xyz");
    }

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

            // Create MCP service
            let mcp_service = StreamableHttpService::new(
                move || Ok(Mosaic::with_shared_serve(serve_clone.clone())),
                LocalSessionManager::default().into(),
                Default::default(),
            );

            // Create CORS layer for all endpoints (MCP + OAuth)
            let cors = build_cors_layer();

            // Conditionally protect MCP endpoints with OAuth middleware + CORS
            let mcp_router = if no_auth {
                Router::new()
                    .nest_service("/mcp", mcp_service)
                    .layer(cors.clone())
            } else {
                Router::new()
                    .nest_service("/mcp", mcp_service)
                    .layer(middleware::from_fn(oauth::oauth_middleware))
                    .layer(cors.clone())
            };

            // OAuth endpoints (publicly accessible)
            let bind_addr_for_metadata = bind_address.clone();
            let oauth_routes = Router::new()
                .route(
                    "/.well-known/oauth-authorization-server",
                    get(move || {
                        oauth::oauth_authorization_server_metadata(bind_addr_for_metadata.clone())
                    }),
                )
                .route("/oauth/register", post(oauth::oauth_register))
                .route("/oauth/token", post(oauth::oauth_token))
                .layer(cors.clone());

            let public_routes = Router::new()
                .route("/assets", get(list_assets_handler))
                .layer(cors);

            // Combine routes
            let router = Router::new()
                .merge(mcp_router)
                .merge(oauth_routes)
                .merge(public_routes);

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
                .route("/desk/{account_id}", get(get_desk_info_handler))
                .route("/desk/{account_id}/note", post(desk_push_note_handler))
                .route("/assets", get(list_assets_handler))
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

    // Validate OAuth environment variables if OAuth is enabled
    if args.mcp
        && !args.no_auth
        && let Err(e) = oauth::validate_auth_env_vars()
    {
        eprintln!("\n❌ OAuth Configuration Error:");
        eprintln!("{}", e);
        eprintln!("\nRequired environment variables:");
        eprintln!("  - MOSAIC_AUTH_DOMAIN (your Auth0 domain)");
        eprintln!("  - MOSAIC_MCP_CLIENT_ID (from 'Mosaic Dev' app in Auth0)");
        eprintln!("  - MOSAIC_MCP_CLIENT_SECRET (from 'Mosaic Dev' app in Auth0)");
        eprintln!("\nPlease set these environment variables or run with --no-auth for testing.\n");
        std::process::exit(1);
    }

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

    println!("{}", helper::MOSAIC_BANNER);

    match (args.mcp, args.rest) {
        (true, false) => {
            // MCP server only
            run_mcp_server(args.mcp_port, args.storage_path, args.no_auth).await?;
        }
        (false, true) => {
            // REST API server only
            run_rest_server(args.rest_port, args.storage_path).await?;
        }
        (true, true) => {
            // Both servers
            if args.mcp_port == args.rest_port {
                // Same port: use combined router for efficiency
                run_combined_server_same_port(args.mcp_port, args.storage_path, args.no_auth)
                    .await?;
            } else {
                // Different ports: run two separate servers concurrently
                run_both_servers_different_ports(
                    args.mcp_port,
                    args.rest_port,
                    args.storage_path,
                    args.no_auth,
                )
                .await?;
            }
        }
        (false, false) => {
            unreachable!("Already validated that at least one server is enabled");
        }
    }

    Ok(())
}
