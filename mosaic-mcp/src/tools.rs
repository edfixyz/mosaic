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
pub struct CreateClientAccountRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateDeskAccountRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
    /// Market information with base and quote currencies
    pub market: mosaic_fi::Market,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateLiquidityAccountRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateFaucetAccountRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
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
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ClientSyncRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreatePrivateNoteRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
    /// Account ID in bech32 format
    pub account_id: String,
    /// Order as JSON object (e.g., {"LiquidityOffer": {"market": "BTC/USD", "uuid": 12345, "amount": 1000, "price": 50000}})
    pub order: mosaic_fi::note::Order,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateNoteFromMasmRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
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
    /// Optional note_secret as 4-element array [u64, u64, u64, u64]
    #[serde(default)]
    pub note_secret: Option<[u64; 4]>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ConsumeNoteRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
    /// Account ID in bech32 format
    pub account_id: String,
    /// Miden note to consume as JSON object
    pub miden_note: mosaic_miden::note::MidenNote,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetAccountStatusRequest {
    /// 32-byte secret as a hex string (64 characters)
    pub secret: String,
    /// Network: "Testnet" or "Localnet"
    pub network: String,
    /// Account ID in bech32 format
    pub account_id: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct AssetInfo {
    /// Faucet account ID in bech32 format
    pub faucet: String,
    /// Amount of the asset
    pub amount: u64,
    /// Whether this is a fungible asset
    pub fungible: bool,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct AccountStatus {
    /// Account ID in bech32 format
    pub account_id: String,
    /// Storage mode: "Private" or "Public"
    #[serde(rename = "type")]
    pub storage_mode: String,
    /// Account type: "Client", "Desk", "Liquidity", or "Faucet"
    pub account_type: String,
    /// List of assets held by the account
    pub assets: Vec<AssetInfo>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct DeskPushNoteRequest {
    /// Desk UUID
    pub desk_uuid: String,
    /// Mosaic note to push to the desk
    pub note: mosaic_fi::note::MosaicNote,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetDeskInfoRequest {
    /// Desk UUID
    pub desk_uuid: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct FlushRequest {}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct VersionRequest {}

// Response types
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CreateClientAccountResponse {
    pub success: bool,
    pub account_id: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CreateDeskAccountResponse {
    pub success: bool,
    pub account_id: String,
    pub desk_uuid: String,
    pub market: mosaic_fi::Market,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CreateLiquidityAccountResponse {
    pub success: bool,
    pub account_id: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CreateFaucetAccountResponse {
    pub success: bool,
    pub account_id: String,
    pub token_symbol: String,
    pub decimals: u8,
    pub max_supply: u64,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct AccountInfo {
    pub account_id: String,
    pub network: String,
    pub account_type: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ListAccountsResponse {
    pub success: bool,
    pub accounts: Vec<AccountInfo>,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ClientSyncResponse {
    pub success: bool,
    pub block_num: u32,
    pub new_public_notes: u32,
    pub committed_notes: u32,
    pub consumed_notes: u32,
    pub updated_accounts: u32,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CreatePrivateNoteResponse {
    pub success: bool,
    pub note: mosaic_fi::note::MosaicNote,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct CreateNoteFromMasmResponse {
    pub success: bool,
    pub note: mosaic_miden::note::MidenNote,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ConsumeNoteResponse {
    pub success: bool,
    pub transaction_id: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct DeskPushNoteResponse {
    pub success: bool,
    pub desk_uuid: String,
    pub note_id: i64,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct GetDeskInfoResponse {
    pub success: bool,
    pub desk_uuid: String,
    pub account_id: String,
    pub network: String,
    pub market: mosaic_fi::Market,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct FlushResponse {
    pub success: bool,
    pub clients_flushed: usize,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct VersionResponse {
    pub success: bool,
    pub version: String,
}

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

    /// Create a new Mosaic instance with a shared Serve instance
    pub fn with_shared_serve(serve: Arc<Mutex<Serve>>) -> Self {
        Self {
            serve,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    #[tool(description = "Create a new Client account")]
    async fn create_client_account(
        &self,
        Parameters(req): Parameters<CreateClientAccountRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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

        // Create the client account
        let account_id_bech32 = {
            let mut serve = self.serve.lock().await;
            serve
                .new_account(secret, AccountType::Client, network)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create client account: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        network = %req.network,
                        "Failed to create client account"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_client_account",
            account_id = %account_id_bech32,
            network = %req.network,
            "Created client account"
        );

        let response = CreateClientAccountResponse {
            success: true,
            account_id: account_id_bech32,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(description = "Create a new Desk account with market information")]
    async fn create_desk_account(
        &self,
        Parameters(req): Parameters<CreateDeskAccountRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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

        // Create the desk account
        let (uuid, account_id_bech32) = {
            let mut serve = self.serve.lock().await;
            serve
                .new_desk_account(secret, network, req.market.clone())
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create desk account: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        network = %req.network,
                        "Failed to create desk account"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_desk_account",
            account_id = %account_id_bech32,
            desk_uuid = %uuid,
            network = %req.network,
            base_currency = %req.market.base.code,
            quote_currency = %req.market.quote.code,
            "Created desk account"
        );

        let response = CreateDeskAccountResponse {
            success: true,
            account_id: account_id_bech32,
            desk_uuid: uuid.to_string(),
            market: req.market,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(description = "Create a new Liquidity account")]
    async fn create_liquidity_account(
        &self,
        Parameters(req): Parameters<CreateLiquidityAccountRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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

        // Create the liquidity account
        let account_id_bech32 = {
            let mut serve = self.serve.lock().await;
            serve
                .new_account(secret, AccountType::Liquidity, network)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to create liquidity account: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        network = %req.network,
                        "Failed to create liquidity account"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "create_liquidity_account",
            account_id = %account_id_bech32,
            network = %req.network,
            "Created liquidity account"
        );

        let response = CreateLiquidityAccountResponse {
            success: true,
            account_id: account_id_bech32,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(
        description = "Create a new faucet account with the specified token symbol, decimals, and max supply"
    )]
    async fn create_faucet_account(
        &self,
        Parameters(req): Parameters<CreateFaucetAccountRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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
                .new_faucet_account(
                    secret,
                    network,
                    req.token_symbol.clone(),
                    req.decimals,
                    req.max_supply,
                )
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

        let response = CreateFaucetAccountResponse {
            success: true,
            account_id: account_id_bech32,
            token_symbol: req.token_symbol,
            decimals: req.decimals,
            max_supply: req.max_supply,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(description = "List all account IDs (bech32) with their networks for a given secret")]
    async fn list_accounts(
        &self,
        Parameters(req): Parameters<ListAccountsRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

        // List accounts
        let accounts = {
            let serve = self.serve.lock().await;
            serve.list_accounts(secret).await.map_err(|e| {
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

        let account_infos: Vec<AccountInfo> = accounts
            .into_iter()
            .map(|(account_id, network, account_type)| AccountInfo {
                account_id,
                network,
                account_type,
            })
            .collect();

        let response = ListAccountsResponse {
            success: true,
            accounts: account_infos,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(description = "Sync a client's state with the network for a given secret and network")]
    async fn client_sync(
        &self,
        Parameters(req): Parameters<ClientSyncRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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
            serve.get_client(secret, network).await.map_err(|e| {
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

        let response = ClientSyncResponse {
            success: true,
            block_num: sync_result.block_num.as_u32(),
            new_public_notes: sync_result.new_public_notes.len() as u32,
            committed_notes: sync_result.committed_notes.len() as u32,
            consumed_notes: sync_result.consumed_notes.len() as u32,
            updated_accounts: sync_result.updated_accounts.len() as u32,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(
        description = "Create a private note from an order for a given secret, network, and account"
    )]
    async fn create_private_note(
        &self,
        Parameters(req): Parameters<CreatePrivateNoteRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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
                .create_private_note(secret, network, req.account_id.clone(), order)
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
            recipient = ?mosaic_note.recipient,
            "Created and committed private note"
        );

        let response = CreatePrivateNoteResponse {
            success: true,
            note: mosaic_note,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(
        description = "Create a note from low-level MASM code and inputs for a given secret, network, and account"
    )]
    async fn create_note_from_masm(
        &self,
        Parameters(req): Parameters<CreateNoteFromMasmRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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
                    secret,
                    network,
                    req.account_id.clone(),
                    note_type,
                    req.program,
                    req.libraries,
                    req.inputs,
                    req.note_secret,
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

        let response = CreateNoteFromMasmResponse {
            success: true,
            note: miden_note,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(
        description = "Get account status including account type and all assets held by the account"
    )]
    async fn get_account_status(
        &self,
        Parameters(req): Parameters<GetAccountStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(error = %error_msg, "Failed to parse secret");
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(error = %error_msg, "Invalid secret length");
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

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

        // Get account status
        let account_status = {
            let mut serve = self.serve.lock().await;
            serve
                .get_account_status(secret, network, req.account_id.clone())
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to get account status: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        account_id = %req.account_id,
                        network = %req.network,
                        "Failed to get account status"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "get_account_status",
            account_id = %req.account_id,
            network = %req.network,
            asset_count = account_status.assets.len(),
            "Retrieved account status"
        );

        // Convert to the MCP response format
        let response = AccountStatus {
            account_id: account_status.account_id,
            storage_mode: account_status.storage_mode,
            account_type: account_status.account_type,
            assets: account_status
                .assets
                .into_iter()
                .map(|a| AssetInfo {
                    faucet: a.faucet,
                    amount: a.amount,
                    fungible: a.fungible,
                })
                .collect(),
        };

        // Serialize to JSON
        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize account status: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize account status");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(
        description = "Consume a note using the specified account. This will execute a transaction to consume the note and add its assets to the account."
    )]
    async fn consume_note(
        &self,
        Parameters(req): Parameters<ConsumeNoteRequest>,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            tool = "consume_note",
            account_id = %req.account_id,
            network = %req.network,
            note_version = %req.miden_note.version,
            note_type = ?req.miden_note.note_type,
            note_hex_length = req.miden_note.miden_note_hex.len(),
            "MCP tools layer: Starting note consumption"
        );

        // Parse hex secret
        let secret_bytes = hex::decode(&req.secret).map_err(|e| {
            let error_msg = format!("Invalid secret hex string: {}", e);
            tracing::error!(
                error = %error_msg,
                account_id = %req.account_id,
                "MCP tools layer: Failed to parse secret"
            );
            McpError::invalid_params(error_msg, None)
        })?;

        if secret_bytes.len() != 32 {
            let error_msg = format!(
                "Secret must be 32 bytes (64 hex chars), got {} bytes",
                secret_bytes.len()
            );
            tracing::error!(
                error = %error_msg,
                secret_length = secret_bytes.len(),
                account_id = %req.account_id,
                "MCP tools layer: Invalid secret length"
            );
            return Err(McpError::invalid_params(error_msg, None));
        }

        let mut secret = [0u8; 32];
        secret.copy_from_slice(&secret_bytes);

        // Parse network
        let network = match req.network.as_str() {
            "Testnet" => Network::Testnet,
            "Localnet" => Network::Localnet,
            _ => {
                let error_msg = format!(
                    "Invalid network '{}'. Must be 'Testnet' or 'Localnet'",
                    req.network
                );
                tracing::error!(
                    error = %error_msg,
                    network = %req.network,
                    account_id = %req.account_id,
                    "MCP tools layer: Invalid network"
                );
                return Err(McpError::invalid_params(error_msg, None));
            }
        };

        tracing::info!(
            account_id = %req.account_id,
            network = ?network,
            "MCP tools layer: Parsed inputs, calling serve layer"
        );

        // Consume the note
        let transaction_id = {
            let mut serve = self.serve.lock().await;
            serve
                .consume_note(secret, network, req.account_id.clone(), req.miden_note)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to consume note: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        account_id = %req.account_id,
                        network = %req.network,
                        "MCP tools layer: Failed to consume note"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "consume_note",
            account_id = %req.account_id,
            network = %req.network,
            transaction_id = %transaction_id,
            "MCP tools layer: Note consumed successfully"
        );

        let response = ConsumeNoteResponse {
            success: true,
            transaction_id,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(description = "Push a Mosaic note to a desk's note store")]
    async fn desk_push_note(
        &self,
        Parameters(req): Parameters<DeskPushNoteRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse UUID
        let desk_uuid = uuid::Uuid::parse_str(&req.desk_uuid).map_err(|e| {
            let error_msg = format!("Invalid desk UUID: {}", e);
            tracing::error!(error = %error_msg, desk_uuid = %req.desk_uuid, "Failed to parse desk UUID");
            McpError::invalid_params(error_msg, None)
        })?;

        // Push the note to the desk
        let note_id = {
            let serve = self.serve.lock().await;
            serve
                .desk_push_note(desk_uuid, req.note.clone())
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to push note to desk: {}", e);
                    tracing::error!(
                        error = %error_msg,
                        desk_uuid = %desk_uuid,
                        "Failed to push note to desk"
                    );
                    McpError::internal_error(error_msg, None)
                })?
        };

        tracing::info!(
            tool = "desk_push_note",
            desk_uuid = %desk_uuid,
            note_id = note_id,
            "Pushed note to desk"
        );

        let response = DeskPushNoteResponse {
            success: true,
            desk_uuid: desk_uuid.to_string(),
            note_id,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
    }

    #[tool(description = "Get desk information including account ID, network, and market data")]
    async fn get_desk_info(
        &self,
        Parameters(req): Parameters<GetDeskInfoRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Parse UUID
        let desk_uuid = uuid::Uuid::parse_str(&req.desk_uuid).map_err(|e| {
            let error_msg = format!("Invalid desk UUID: {}", e);
            tracing::error!(error = %error_msg, desk_uuid = %req.desk_uuid, "Failed to parse desk UUID");
            McpError::invalid_params(error_msg, None)
        })?;

        // Get desk info
        let (account_id, network, market) = {
            let serve = self.serve.lock().await;
            serve.get_desk_info(desk_uuid).await.map_err(|e| {
                let error_msg = format!("Failed to get desk info: {}", e);
                tracing::error!(
                    error = %error_msg,
                    desk_uuid = %desk_uuid,
                    "Failed to get desk info"
                );
                McpError::internal_error(error_msg, None)
            })?
        };

        tracing::info!(
            tool = "get_desk_info",
            desk_uuid = %desk_uuid,
            account_id = %account_id,
            "Retrieved desk info"
        );

        let response = GetDeskInfoResponse {
            success: true,
            desk_uuid: desk_uuid.to_string(),
            account_id,
            network: match network {
                Network::Testnet => "Testnet".to_string(),
                Network::Localnet => "Localnet".to_string(),
            },
            market,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
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

        let response = FlushResponse {
            success: true,
            clients_flushed: client_count,
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
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

        let response = VersionResponse {
            success: true,
            version: version.to_string(),
        };

        let response_json = serde_json::to_string_pretty(&response).map_err(|e| {
            let error_msg = format!("Failed to serialize response: {}", e);
            tracing::error!(error = %error_msg, "Failed to serialize response");
            McpError::internal_error(error_msg, None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(response_json)]))
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
            instructions: Some("Mosaic MCP server. Tools: create_account - Create a new Mosaic account with a 32-byte secret (hex string), account type (Client, Desk, or Liquidity), and network (Testnet or Localnet); list_accounts - List all account IDs (bech32) with their networks for a given secret; client_sync - Sync a client's state with the network; create_private_note - Create a private note from an order for a given secret, network, and account.".to_string()),
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
