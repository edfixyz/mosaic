#![allow(dead_code)]
use mosaic_fi::AccountType;
use mosaic_miden::Network;
use mosaic_serve::Serve;
use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    handler::server::{
        router::{prompt::PromptRouter, tool::ToolRouter},
        wrapper::Parameters,
    },
    model::*,
    prompt_handler, prompt_router, schemars,
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use tokio::sync::Mutex;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateAccountRequest {
    /// 32-byte identifier as a hex string (64 characters)
    pub identifier: String,
    /// Account type: "Client", "Desk", or "Liquidity"
    pub account_type: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateFaucetAccountRequest {
    /// 32-byte identifier as a hex string (64 characters)
    pub identifier: String,
    /// Token symbol (e.g., "MID")
    pub token_symbol: String,
    /// Number of decimals for the token
    pub decimals: u8,
    /// Maximum supply of tokens
    pub max_supply: u64,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ListAccountsRequest {
    /// 32-byte identifier as a hex string (64 characters)
    pub identifier: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ClientSyncRequest {
    /// 32-byte identifier as a hex string (64 characters)
    pub identifier: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreatePrivateNoteRequest {
    /// 32-byte identifier as a hex string (64 characters)
    pub identifier: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
    /// Account ID in bech32 format
    pub account_id: String,
    /// Order as JSON object (e.g., {"LiquidityOffer": {"market": "BTC/USD", "uuid": 12345, "amount": 1000, "price": 50000}})
    pub order: mosaic_fi::note::Order,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateNoteFromMasmRequest {
    /// 32-byte identifier as a hex string (64 characters)
    pub identifier: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
    /// Account ID in bech32 format
    pub account_id: String,
    /// Note type: "Private" or "Public"
    pub note_type: String,
    /// Miden assembly program source code
    pub program: String,
    /// Optional external libraries as array of [name, source] pairs
    #[serde(default)]
    pub libraries: Vec<(String, String)>,
    /// Optional inputs as array of [name, value] pairs where value is {"Word": [u64, u64, u64, u64]} or {"Element": u64}
    #[serde(default)]
    pub inputs: Vec<(String, mosaic_miden::note::Value)>,
    /// Optional secret as 4-element array [u64, u64, u64, u64]
    #[serde(default)]
    pub secret: Option<[u64; 4]>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FlushRequest {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionRequest {}

#[derive(Clone)]
pub struct Mosaic {
    serve: Arc<Mutex<Serve>>,
    tool_router: ToolRouter<Mosaic>,
    prompt_router: PromptRouter<Mosaic>,
}

#[tool_router]
impl Mosaic {
    #[allow(dead_code)]
    pub fn new(serve: Serve) -> Self {
        Self {
            serve: Arc::new(Mutex::new(serve)),
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    #[tool(description = "Create a new Mosaic account with the specified identifier and type")]
    async fn create_account(
        &self,
        Parameters(req): Parameters<CreateAccountRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex identifier
        let identifier_bytes = hex::decode(&req.identifier).map_err(|e| {
            let error_msg = format!("Invalid identifier hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse identifier");
            McpError::invalid_params(error_msg, None)
        })?;

        if identifier_bytes.len() != 32 {
            let error_msg = format!(
                "Identifier must be 32 bytes (64 hex chars), got {} bytes",
                identifier_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid identifier length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut identifier = [0u8; 32];
        identifier.copy_from_slice(&identifier_bytes);

        // Parse account type
        let account_type = match req.account_type.as_str() {
            "Client" => AccountType::Client,
            "Desk" => AccountType::Desk,
            "Liquidity" => AccountType::Liquidity,
            _ => {
                let error_msg = format!(
                    "Invalid account type '{}'. Must be 'Client', 'Desk', or 'Liquidity'",
                    req.account_type
                );
                tracing::error!(error = %error_msg, account_type = %req.account_type, "Invalid account type");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Parse network
        let network = match req.network.as_str() {
            "Testnet" => Network::Testnet,
            "Localnet" => Network::Localnet,
            _ => {
                let error_msg = format!(
                    "Invalid network '{}'. Must be 'Testnet' or 'Localnet'",
                    req.network
                );
                tracing::error!(error = %error_msg, network = %req.network, "Invalid network");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Create the account
        let account_id_bech32 = {
            let mut serve = self.serve.lock().await;
            serve
                .new_account(identifier, account_type, network)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create account: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        account_type = %req.account_type,
                        network = %req.network,
                        "Failed to create account"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_account",
            account_id = %account_id_bech32,
            account_type = %req.account_type,
            network = %req.network,
            "Created account"
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Account created successfully!\nIdentifier: {}\nAccount ID (bech32): {}",
            req.identifier, account_id_bech32
        ))]))
    }

    #[tool(description = "Create a new faucet account with the specified token symbol, decimals, and max supply")]
    async fn create_faucet_account(
        &self,
        Parameters(req): Parameters<CreateFaucetAccountRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex identifier
        let identifier_bytes = hex::decode(&req.identifier).map_err(|e| {
            let error_msg = format!("Invalid identifier hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse identifier");
            McpError::invalid_params(error_msg, None)
        })?;

        if identifier_bytes.len() != 32 {
            let error_msg = format!(
                "Identifier must be 32 bytes (64 hex chars), got {} bytes",
                identifier_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid identifier length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut identifier = [0u8; 32];
        identifier.copy_from_slice(&identifier_bytes);

        // Parse network
        let network = match req.network.as_str() {
            "Testnet" => Network::Testnet,
            "Localnet" => Network::Localnet,
            _ => {
                let error_msg = format!(
                    "Invalid network '{}'. Must be 'Testnet' or 'Localnet'",
                    req.network
                );
                tracing::error!(error = %error_msg, network = %req.network, "Invalid network");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Create the faucet account
        let account_id_bech32 = {
            let mut serve = self.serve.lock().await;
            serve
                .new_faucet_account(identifier, network, req.token_symbol.clone(), req.decimals, req.max_supply)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create faucet account: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        token_symbol = %req.token_symbol,
                        network = %req.network,
                        "Failed to create faucet account"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_faucet_account",
            account_id = %account_id_bech32,
            token_symbol = %req.token_symbol,
            decimals = req.decimals,
            max_supply = req.max_supply,
            network = %req.network,
            "Created faucet account"
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Faucet account created successfully!\nIdentifier: {}\nAccount ID (bech32): {}\nToken: {} (decimals: {}, max supply: {})",
            req.identifier, account_id_bech32, req.token_symbol, req.decimals, req.max_supply
        ))]))
    }

    #[tool(
        description = "List all account IDs (bech32) with their networks for a given identifier"
    )]
    async fn list_accounts(
        &self,
        Parameters(req): Parameters<ListAccountsRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex identifier
        let identifier_bytes = hex::decode(&req.identifier).map_err(|e| {
            let error_msg = format!("Invalid identifier hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse identifier");
            McpError::invalid_params(error_msg, None)
        })?;

        if identifier_bytes.len() != 32 {
            let error_msg = format!(
                "Identifier must be 32 bytes (64 hex chars), got {} bytes",
                identifier_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid identifier length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut identifier = [0u8; 32];
        identifier.copy_from_slice(&identifier_bytes);

        // List accounts
        let accounts = {
            let serve = self.serve.lock().await;
            serve.list_accounts(identifier).await.map_err(|e| {
                let error_msg = format!("Failed to list accounts: {}", e);
                tracing::error!(error = %error_msg, "Failed to list accounts");
                McpError::internal_error(error_msg, None)
            })?
        };

        tracing::info!(
            tool = "list_accounts",
            account_count = accounts.len(),
            "Listed accounts"
        );

        if accounts.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No accounts found for identifier: {}",
                req.identifier
            ))]));
        }

        let mut response = format!("Accounts for identifier {}:\n\n", req.identifier);
        for (i, (account_id, network)) in accounts.iter().enumerate() {
            response.push_str(&format!(
                "{}. Account ID: {}\n   Network: {}\n\n",
                i + 1,
                account_id,
                network
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(response)]))
    }

    #[tool(
        description = "Sync a client's state with the network for a given identifier and network"
    )]
    async fn client_sync(
        &self,
        Parameters(req): Parameters<ClientSyncRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex identifier
        let identifier_bytes = hex::decode(&req.identifier).map_err(|e| {
            let error_msg = format!("Invalid identifier hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse identifier");
            McpError::invalid_params(error_msg, None)
        })?;

        if identifier_bytes.len() != 32 {
            let error_msg = format!(
                "Identifier must be 32 bytes (64 hex chars), got {} bytes",
                identifier_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid identifier length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut identifier = [0u8; 32];
        identifier.copy_from_slice(&identifier_bytes);

        // Parse network
        let network = match req.network.as_str() {
            "Testnet" => Network::Testnet,
            "Localnet" => Network::Localnet,
            _ => {
                let error_msg = format!(
                    "Invalid network '{}'. Must be 'Testnet' or 'Localnet'",
                    req.network
                );
                tracing::error!(error = %error_msg, network = %req.network, "Invalid network");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Get client handle
        let client_handle = {
            let mut serve = self.serve.lock().await;
            serve.get_client(identifier, network).await.map_err(|e| {
                let error_msg = format!("Failed to get client: {}", e);
                tracing::error!(error = %error_msg, network = %req.network, "Failed to get client");
                McpError::internal_error(error_msg, None)
            })?
        };

        // Sync the client state - ClientHandle.sync() is Send-safe!
        let sync_result = client_handle.sync().await.map_err(|e| {
            let error_msg = format!("Failed to sync state: {}", e);
            tracing::error!(error = %error_msg, network = %req.network, "Failed to sync state");
            McpError::internal_error(error_msg, None)
        })?;

        tracing::info!(
            tool = "client_sync",
            network = %req.network,
            block_num = %sync_result.block_num,
            new_public_notes = sync_result.new_public_notes.len(),
            committed_notes = sync_result.committed_notes.len(),
            consumed_notes = sync_result.consumed_notes.len(),
            updated_accounts = sync_result.updated_accounts.len(),
            "Client synced"
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Client synced successfully!\nIdentifier: {}\nNetwork: {}\nBlock: {}\nNew public notes: {}\nCommitted notes: {}\nConsumed notes: {}\nUpdated accounts: {}",
            req.identifier,
            req.network,
            sync_result.block_num,
            sync_result.new_public_notes.len(),
            sync_result.committed_notes.len(),
            sync_result.consumed_notes.len(),
            sync_result.updated_accounts.len()
        ))]))
    }

    #[tool(
        description = "Create a private note from an order for a given identifier, network, and account"
    )]
    async fn create_private_note(
        &self,
        Parameters(req): Parameters<CreatePrivateNoteRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex identifier
        let identifier_bytes = hex::decode(&req.identifier).map_err(|e| {
            let error_msg = format!("Invalid identifier hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse identifier");
            McpError::invalid_params(error_msg, None)
        })?;

        if identifier_bytes.len() != 32 {
            let error_msg = format!(
                "Identifier must be 32 bytes (64 hex chars), got {} bytes",
                identifier_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid identifier length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut identifier = [0u8; 32];
        identifier.copy_from_slice(&identifier_bytes);

        // Parse network
        let network = match req.network.as_str() {
            "Testnet" => Network::Testnet,
            "Localnet" => Network::Localnet,
            _ => {
                let error_msg = format!(
                    "Invalid network '{}'. Must be 'Testnet' or 'Localnet'",
                    req.network
                );
                tracing::error!(error = %error_msg, network = %req.network, "Invalid network");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Parse order from JSON
        let order: mosaic_fi::note::Order = req.order;

        // Create the note
        let mosaic_note = {
            let mut serve = self.serve.lock().await;
            serve
                .create_private_note(identifier, network, req.account_id.clone(), order)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create private note: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        account_id = %req.account_id,
                        network = %req.network,
                        "Failed to create private note"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_private_note",
            account_id = %req.account_id,
            network = %req.network,
            market = %mosaic_note.market,
            "Created and committed private note"
        );

        // Serialize the note to JSON for the response
        let note_json = serde_json::to_string_pretty(&mosaic_note).map_err(|e| {
            let error_msg = format!("Failed to serialize note: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize note");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Private note created successfully!\nIdentifier: {}\nNetwork: {}\nAccount ID: {}\n\nNote:\n{}",
            req.identifier, req.network, req.account_id, note_json
        ))]))
    }

    #[tool(
        description = "Create a note from low-level MASM code and inputs for a given identifier, network, and account"
    )]
    async fn create_note_from_masm(
        &self,
        Parameters(req): Parameters<CreateNoteFromMasmRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex identifier
        let identifier_bytes = hex::decode(&req.identifier).map_err(|e| {
            let error_msg = format!("Invalid identifier hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse identifier");
            McpError::invalid_params(error_msg, None)
        })?;

        if identifier_bytes.len() != 32 {
            let error_msg = format!(
                "Identifier must be 32 bytes (64 hex chars), got {} bytes",
                identifier_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid identifier length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut identifier = [0u8; 32];
        identifier.copy_from_slice(&identifier_bytes);

        // Parse network
        let network = match req.network.as_str() {
            "Testnet" => Network::Testnet,
            "Localnet" => Network::Localnet,
            _ => {
                let error_msg = format!(
                    "Invalid network '{}'. Must be 'Testnet' or 'Localnet'",
                    req.network
                );
                tracing::error!(error = %error_msg, network = %req.network, "Invalid network");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Parse note type
        let note_type = match req.note_type.as_str() {
            "Private" => mosaic_miden::note::NoteType::Private,
            "Public" => {
                let error_msg = "Public notes are not supported yet";
                tracing::error!(error = %error_msg, note_type = %req.note_type, "Unsupported note type");
                return Err(McpError::invalid_params(error_msg.to_string(), None));
            }
            _ => {
                let error_msg = format!(
                    "Invalid note type '{}'. Must be 'Private' or 'Public'",
                    req.note_type
                );
                tracing::error!(error = %error_msg, note_type = %req.note_type, "Invalid note type");
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        // Create the note
        let miden_note = {
            let mut serve = self.serve.lock().await;
            serve
                .create_note_from_masm(
                    identifier,
                    network,
                    req.account_id.clone(),
                    note_type,
                    req.program,
                    req.libraries,
                    req.inputs,
                    req.secret,
                )
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create note from MASM: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        account_id = %req.account_id,
                        network = %req.network,
                        "Failed to create note from MASM"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_note_from_masm",
            account_id = %req.account_id,
            network = %req.network,
            note_type = %req.note_type,
            "Created and committed note from MASM"
        );

        // Serialize the note to JSON for the response
        let note_json = serde_json::to_string_pretty(&miden_note).map_err(|e| {
            let error_msg = format!("Failed to serialize note: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize note");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Note created from MASM successfully!\nIdentifier: {}\nNetwork: {}\nAccount ID: {}\n\nNote:\n{}",
            req.identifier, req.network, req.account_id, note_json
        ))]))
    }

    #[tool(description = "Flush all cached clients and in-memory objects")]
    async fn flush(
        &self,
        Parameters(_req): Parameters<FlushRequest>,
    ) -> Result<CallToolResult, McpError> {
        let client_count = {
            let mut serve = self.serve.lock().await;
            serve.flush()
        };

        tracing::info!(
            tool = "flush",
            clients_flushed = client_count,
            "Flushed cache"
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Cache flushed successfully!\nClients cleared: {}",
            client_count
        ))]))
    }

    #[tool(description = "Get the current Mosaic version string")]
    async fn version(
        &self,
        Parameters(_req): Parameters<VersionRequest>,
    ) -> Result<CallToolResult, McpError> {
        let version = mosaic_miden::version::VERSION_STRING;

        tracing::info!(
            tool = "version",
            version = %version,
            "Version requested"
        );

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Mosaic version: {}",
            version
        ))]))
    }
}

#[prompt_router]
impl Mosaic {}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for Mosaic {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("Mosaic MCP server. Tools: create_account - Create a new Mosaic account with a 32-byte identifier (hex string), account type (Client, Desk, or Liquidity), and network (Testnet or Localnet); list_accounts - List all account IDs (bech32) with their networks for a given identifier; client_sync - Sync a client's state with the network; create_private_note - Create a private note from an order for a given identifier, network, and account.".to_string()),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        ReadResourceRequestParam { uri }: ReadResourceRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::resource_not_found(
            "resource_not_found",
            Some(serde_json::json!({
                "uri": uri
            })),
        ))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            next_cursor: None,
            resource_templates: Vec::new(),
        })
    }

    async fn initialize(
        &self,
        _request: InitializeRequestParam,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        if let Some(http_request_part) = context.extensions.get::<axum::http::request::Parts>() {
            let initialize_headers = &http_request_part.headers;
            let initialize_uri = &http_request_part.uri;
            tracing::info!(?initialize_headers, %initialize_uri, "initialize from http server");
        }
        Ok(self.get_info())
    }
}
