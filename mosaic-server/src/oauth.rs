use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

// Validate required environment variables at startup
pub fn validate_auth_env_vars() -> Result<(), String> {
    let required_vars = [
        "MOSAIC_AUTH_DOMAIN",
        "MOSAIC_MCP_CLIENT_ID",
        "MOSAIC_MCP_CLIENT_SECRET",
    ];

    let mut missing = Vec::new();

    for var in &required_vars {
        if std::env::var(var).is_err() {
            missing.push(*var);
        }
    }

    if !missing.is_empty() {
        return Err(format!(
            "Missing required environment variables: {}",
            missing.join(", ")
        ));
    }

    Ok(())
}

// Environment variable helpers
fn get_auth_domain() -> String {
    std::env::var("MOSAIC_AUTH_DOMAIN")
        .expect("MOSAIC_AUTH_DOMAIN environment variable must be set")
}

fn get_mcp_client_id() -> String {
    std::env::var("MOSAIC_MCP_CLIENT_ID")
        .expect("MOSAIC_MCP_CLIENT_ID environment variable must be set")
}

fn get_mcp_client_secret() -> String {
    std::env::var("MOSAIC_MCP_CLIENT_SECRET")
        .expect("MOSAIC_MCP_CLIENT_SECRET environment variable must be set")
}

fn get_public_server_url() -> String {
    std::env::var("MOSAIC_SERVER").unwrap_or_else(|_| {
        warn!(
            "MOSAIC_SERVER environment variable is not set; falling back to local server URL for WWW-Authenticate header"
        );
        "http://localhost:8000".to_string()
    })
}

// API audience (can be hardcoded as it's not sensitive)
const AUTH0_AUDIENCE: &str = "https://api.mosaic.edfi.xyz";

// JWKS cache with lazy initialization
static JWKS_CACHE: Lazy<Arc<RwLock<JwksCache>>> = Lazy::new(|| {
    Arc::new(RwLock::new(JwksCache {
        keys: HashMap::new(),
        last_fetch: None,
    }))
});

// Client storage for dynamically registered clients
static CLIENT_STORAGE: Lazy<Arc<RwLock<HashMap<String, RegisteredClient>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

// Recently validated tokens cache (token -> timestamp)
#[derive(Clone)]
struct CachedToken {
    claims: Claims,
    validated_at: std::time::Instant,
}

static VALIDATED_TOKENS: Lazy<Arc<RwLock<HashMap<String, CachedToken>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Clone, Debug)]
struct RegisteredClient {
    #[allow(dead_code)]
    client_id: String,
    #[allow(dead_code)]
    client_secret: String,
    #[allow(dead_code)]
    redirect_uris: Vec<String>,
    #[allow(dead_code)]
    created_at: std::time::Instant,
}

#[derive(Clone)]
struct JwksCache {
    keys: HashMap<String, DecodingKey>,
    last_fetch: Option<std::time::Instant>,
}

impl JwksCache {
    fn should_refresh(&self) -> bool {
        match self.last_fetch {
            None => true,
            Some(last) => last.elapsed() > std::time::Duration::from_secs(3600), // Refresh every hour
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    #[serde(rename = "use")]
    key_use: Option<String>,
    n: String,
    e: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    aud: Audience,
    exp: usize,
    iat: usize,
    iss: String,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

// Fetch JWKS from Auth0
async fn fetch_jwks() -> Result<HashMap<String, DecodingKey>, Box<dyn std::error::Error>> {
    let auth_domain = get_auth_domain();
    let jwks_url = format!("https://{}/.well-known/jwks.json", auth_domain);
    debug!("Fetching JWKS from: {}", jwks_url);

    let response = reqwest::get(&jwks_url).await?;
    let jwks: Jwks = response.json().await?;

    let mut keys = HashMap::new();
    for key in jwks.keys {
        if key.kty == "RSA" {
            let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)?;
            keys.insert(key.kid, decoding_key);
        }
    }

    info!("Loaded {} JWKS keys from Auth0", keys.len());
    Ok(keys)
}

// Get or refresh JWKS cache
async fn get_jwks() -> Result<HashMap<String, DecodingKey>, Box<dyn std::error::Error>> {
    {
        let cache = JWKS_CACHE.read().await;

        if !cache.should_refresh() {
            return Ok(cache.keys.clone());
        }
    } // Read lock is released here

    // Need to refresh
    let mut cache = JWKS_CACHE.write().await;

    // Double-check in case another task already refreshed
    if cache.should_refresh() {
        let keys = fetch_jwks().await?;
        cache.keys = keys.clone();
        cache.last_fetch = Some(std::time::Instant::now());
        return Ok(keys);
    }

    Ok(cache.keys.clone())
}

// Validate token from Auth0 (handles both JWT and opaque tokens)
async fn validate_token(token: &str) -> Result<Claims, String> {
    // Check cache first to avoid rate limits
    if let Some(cached) = VALIDATED_TOKENS.read().await.get(token).cloned()
        && cached.validated_at.elapsed() < std::time::Duration::from_secs(300)
    {
        debug!("✨ Token found in cache, skipping validation");
        return Ok(cached.claims);
    }

    // Not in cache or expired - validate
    let claims = match decode_header(token) {
        Ok(header) => {
            // It's a JWT - validate with JWKS
            debug!("Token is a JWT, validating with JWKS");
            validate_jwt_token(token, header).await?
        }
        Err(_) => {
            // Not a JWT - must be an opaque token
            // Validate by calling Auth0's /userinfo endpoint
            debug!("Token appears to be opaque, validating with /userinfo endpoint");
            validate_opaque_token(token).await?
        }
    };

    // Cache the validated token
    VALIDATED_TOKENS.write().await.insert(
        token.to_string(),
        CachedToken {
            claims: claims.clone(),
            validated_at: std::time::Instant::now(),
        },
    );
    debug!("💾 Token cached for future requests");

    Ok(claims)
}

// Validate JWT token using JWKS
async fn validate_jwt_token(token: &str, header: jsonwebtoken::Header) -> Result<Claims, String> {
    let kid = header.kid.ok_or("Missing kid in token header")?;

    // Get JWKS
    let jwks = get_jwks()
        .await
        .map_err(|e| format!("Failed to get JWKS: {}", e))?;

    // Get decoding key for this kid
    let decoding_key = jwks
        .get(&kid)
        .ok_or_else(|| format!("Unknown kid: {}", kid))?;

    // Set up validation
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[AUTH0_AUDIENCE]);
    let auth_domain = get_auth_domain();
    validation.set_issuer(&[format!("https://{}/", auth_domain)]);

    // Decode and validate token
    let token_data = decode::<Claims>(token, decoding_key, &validation)
        .map_err(|e| format!("Token validation failed: {}", e))?;

    Ok(token_data.claims)
}

// Validate opaque token by calling Auth0's /userinfo endpoint
async fn validate_opaque_token(token: &str) -> Result<Claims, String> {
    let auth_domain = get_auth_domain();
    let client = reqwest::Client::new();

    let response = client
        .get(format!("https://{}/userinfo", auth_domain))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Failed to call /userinfo: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        error!("❌ /userinfo failed with HTTP {}: {}", status, error_body);
        return Err(format!(
            "Token validation failed: HTTP {} - {}",
            status, error_body
        ));
    }

    // Parse userinfo response
    let userinfo: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse /userinfo response: {}", e))?;

    debug!(
        "📋 Userinfo response: {}",
        serde_json::to_string_pretty(&userinfo).unwrap_or_default()
    );

    // Extract sub (user ID) from userinfo
    // If userinfo is empty, the token likely doesn't have 'openid' scope
    // For MCP, we'll accept this since token was just exchanged successfully
    let sub = if let Some(sub_value) = userinfo.get("sub").and_then(|v| v.as_str()) {
        sub_value.to_string()
    } else {
        // Empty userinfo response - likely missing 'openid' scope
        // Use token hash as pseudo-user-id for tracking purposes
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        token.hash(&mut hasher);
        let token_hash = format!("token_{:x}", hasher.finish());
        info!(
            "⚠️  Userinfo empty (missing 'openid' scope), accepting token anyway. Token hash: {}",
            token_hash
        );
        token_hash
    };

    info!("✅ Opaque token validated for user: {}", sub);

    let auth_domain = get_auth_domain();

    // Create minimal claims for opaque tokens
    Ok(Claims {
        sub,
        aud: Audience::Single(AUTH0_AUDIENCE.to_string()),
        exp: 0, // We don't know expiry for opaque tokens, rely on Auth0's validation
        iat: 0, // Not available for opaque tokens
        iss: format!("https://{}/", auth_domain),
        extra: HashMap::new(),
    })
}

// Auth middleware for protecting MCP endpoints
pub async fn oauth_middleware(request: Request<Body>, next: Next) -> Response {
    debug!("OAuth middleware: checking request {:?}", request.uri());

    // Allow OPTIONS requests (CORS preflight) without auth
    if request.method() == axum::http::Method::OPTIONS {
        return next.run(request).await;
    }

    // Try to get token from Authorization header first, then query param
    let token = if let Some(auth_header) = request.headers().get("Authorization") {
        // Extract from Authorization header
        let auth_str = match auth_header.to_str() {
            Ok(s) => s,
            Err(_) => {
                error!("Invalid Authorization header");
                return create_auth_challenge_response("Invalid Authorization header");
            }
        };

        match auth_str.strip_prefix("Bearer ") {
            Some(t) => t.to_string(),
            None => {
                error!("Authorization header must start with 'Bearer '");
                return create_auth_challenge_response(
                    "Authorization header must start with 'Bearer '",
                );
            }
        }
    } else if let Some(query) = request.uri().query() {
        // Try to extract from query parameter (for EventSource connections)
        let params: Vec<&str> = query.split('&').collect();
        let mut token_value = None;

        for param in params {
            if let Some(value) = param.strip_prefix("access_token=") {
                token_value = Some(value.to_string());
                break;
            }
        }

        match token_value {
            Some(t) => {
                debug!("Token extracted from query parameter");
                t
            }
            None => {
                debug!("Missing Authorization header and access_token query param");
                return create_auth_challenge_response("Bearer token required");
            }
        }
    } else {
        debug!("Missing Authorization header and no query params");
        return create_auth_challenge_response("Bearer token required");
    };

    // Validate token
    match validate_token(&token).await {
        Ok(claims) => {
            debug!("Token validated for user: {}", claims.sub);

            // Format user ID as mosaic|auth0|{sub} or mosaic|token|{hash}
            let user_id = if claims.sub.starts_with("token_") {
                // Opaque token without openid scope
                format!("mosaic|{}", claims.sub) // mosaic|token_abc123
            } else if claims.sub.starts_with("cached_token_") {
                // Cached token
                format!("mosaic|{}", claims.sub.replace("cached_", "")) // mosaic|token_abc123
            } else {
                // Real Auth0 user ID
                format!("mosaic|auth0|{}", claims.sub) // mosaic|auth0|auth0|123456789
            };

            info!("🔐 Authenticated user: {}", user_id);

            // Store user_id in request extensions for MCP handlers to access
            let mut request = request;
            request.extensions_mut().insert(user_id);

            // Token is valid, continue to the next middleware/handler
            next.run(request).await
        }
        Err(e) => {
            error!("Token validation failed: {}", e);
            create_auth_challenge_response(&format!("Invalid token: {}", e))
        }
    }
}

// Create a proper OAuth challenge response
fn create_auth_challenge_response(error_description: &str) -> Response {
    use axum::http::header;

    let auth_domain = get_auth_domain();
    let public_url = get_public_server_url();
    let resource_metadata = format!(
        "{}/.well-known/oauth-protected-resource",
        public_url.trim_end_matches('/')
    );
    let www_authenticate = format!(
        r#"Bearer resource_metadata="{}" scopes="openid profile email""#,
        resource_metadata
    );

    (
        StatusCode::UNAUTHORIZED,
        [
            (header::WWW_AUTHENTICATE, www_authenticate),
            (header::CONTENT_TYPE, "application/json".to_string()),
        ],
        Json(serde_json::json!({
            "error": "unauthorized",
            "error_description": error_description,
            "authorization_uri": format!("https://{}/authorize", auth_domain),
        })),
    )
        .into_response()
}

// OAuth metadata endpoint
#[derive(Debug, Serialize)]
pub struct AuthorizationMetadata {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub issuer: String,
    pub jwks_uri: String,
    pub registration_endpoint: String,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

pub async fn oauth_authorization_server_metadata(_bind_addr: String) -> impl IntoResponse {
    let auth_domain = get_auth_domain();
    let public_url = get_public_server_url();
    let metadata = AuthorizationMetadata {
        // Auth0 endpoints for actual OAuth flow
        authorization_endpoint: format!("https://{}/authorize", auth_domain),
        token_endpoint: format!("{}/oauth/token", public_url.trim_end_matches('/')),
        issuer: format!("https://{}/", auth_domain),
        jwks_uri: format!("https://{}/.well-known/jwks.json", auth_domain),
        // Our registration endpoint
        registration_endpoint: format!("{}/oauth/register", public_url.trim_end_matches('/')),
        scopes_supported: vec![
            "openid".to_string(),
            "profile".to_string(),
            "email".to_string(),
        ],
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "refresh_token".to_string(),
        ],
        token_endpoint_auth_methods_supported: vec![
            "none".to_string(), // PKCE-only public clients
            "client_secret_basic".to_string(),
            "client_secret_post".to_string(),
        ],
        code_challenge_methods_supported: vec!["S256".to_string()],
    };

    (StatusCode::OK, Json(metadata))
}

#[derive(Debug, Serialize)]
struct Auth0CodeExchangeRequest {
    grant_type: String,
    client_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    client_secret: String,
    code: String,
    redirect_uri: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    code_verifier: String,
}

// MCP Client Registration structures
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub redirect_uris: Vec<String>,
    pub client_name: Option<String>,
    #[serde(flatten)]
    #[allow(dead_code)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    pub redirect_uris: Vec<String>,
    #[serde(flatten)]
    pub additional_fields: HashMap<String, serde_json::Value>,
}

// Client registration endpoint for MCP
pub async fn oauth_register(Json(req): Json<ClientRegistrationRequest>) -> impl IntoResponse {
    info!("📝 Client registration request received");
    debug!("Registration details: {:?}", req);

    if req.redirect_uris.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_request",
                "error_description": "at least one redirect_uri is required"
            })),
        )
            .into_response();
    }

    // Use static MCP client credentials instead of dynamic registration
    let client_id = get_mcp_client_id();
    let client_secret = get_mcp_client_secret();
    let client_name = req
        .client_name
        .clone()
        .unwrap_or_else(|| "Mosaic MCP".to_string());

    // Store client info locally for token validation
    let registered_client = RegisteredClient {
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_uris: req.redirect_uris.clone(),
        created_at: std::time::Instant::now(),
    };

    CLIENT_STORAGE
        .write()
        .await
        .insert(client_id.clone(), registered_client);
    info!("✅ Registered static MCP client: {}", client_id);

    let response = ClientRegistrationResponse {
        client_id,
        client_secret,
        client_name: Some(client_name),
        redirect_uris: req.redirect_uris,
        additional_fields: HashMap::new(),
    };

    (StatusCode::CREATED, Json(response)).into_response()
}

// Token exchange structures
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default)]
    pub code_verifier: String,
}

// Token exchange endpoint
pub async fn oauth_token(request: axum::http::Request<Body>) -> impl IntoResponse {
    info!("Token exchange request received");

    let bytes = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "cannot read request body"
                })),
            )
                .into_response();
        }
    };

    // Log raw request body for debugging
    if let Ok(body_str) = std::str::from_utf8(&bytes) {
        debug!("📨 Raw token request body: {}", body_str);
    }

    let token_req = match serde_urlencoded::from_bytes::<TokenRequest>(&bytes) {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to parse token request: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "error_description": format!("invalid form data: {}", e)
                })),
            )
                .into_response();
        }
    };

    if token_req.grant_type != "authorization_code" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "unsupported_grant_type",
                "error_description": "only authorization_code is supported"
            })),
        )
            .into_response();
    }

    // For PKCE-only flows, client_id might be empty - use our static client_id
    let client_id = if token_req.client_id.is_empty() {
        info!("🔐 PKCE-only flow detected (no client_id in request)");
        get_mcp_client_id()
    } else {
        info!(
            "🔍 Token exchange request for client_id: {}",
            token_req.client_id
        );
        token_req.client_id.clone()
    };

    info!("📦 Using client_id: {}", client_id);

    // Exchange code with Auth0
    let client = reqwest::Client::new();
    let auth_domain = get_auth_domain();
    let exchange_request = Auth0CodeExchangeRequest {
        grant_type: "authorization_code".to_string(),
        client_id: client_id.clone(),
        client_secret: token_req.client_secret.clone(), // Empty for PKCE-only
        code: token_req.code.clone(),
        redirect_uri: token_req.redirect_uri.clone(),
        code_verifier: token_req.code_verifier.clone(), // PKCE verifier
    };

    debug!(
        "📤 Sending to Auth0: grant_type={}, client_id={}, has_verifier={}, has_secret={}",
        exchange_request.grant_type,
        exchange_request.client_id,
        !exchange_request.code_verifier.is_empty(),
        !exchange_request.client_secret.is_empty()
    );

    let response = match client
        .post(format!("https://{}/oauth/token", auth_domain))
        .json(&exchange_request)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to exchange code with Auth0: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "server_error",
                    "error_description": "failed to exchange authorization code"
                })),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        error!("Auth0 token exchange failed: {}", error_text);
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "authorization code exchange failed"
            })),
        )
            .into_response();
    }

    // Forward the token response from Auth0
    match response.json::<serde_json::Value>().await {
        Ok(token_data) => {
            info!("Successfully exchanged code for token");
            (StatusCode::OK, Json(token_data)).into_response()
        }
        Err(e) => {
            error!("Failed to parse Auth0 token response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": "server_error",
                    "error_description": "failed to parse token response"
                })),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwks_cache_should_refresh() {
        let cache = JwksCache {
            keys: HashMap::new(),
            last_fetch: None,
        };
        assert!(cache.should_refresh());

        let cache = JwksCache {
            keys: HashMap::new(),
            last_fetch: Some(std::time::Instant::now()),
        };
        assert!(!cache.should_refresh());
    }
}
