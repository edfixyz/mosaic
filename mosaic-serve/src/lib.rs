use mosaic_fi::note::{MosaicNote, MosaicNoteStatus};
use mosaic_fi::{AccountOrder, AccountOrderResult, AccountType, Market};
use mosaic_miden::client::ClientHandle;
use mosaic_miden::store::{AssetRecord, OrderRecord, SettingsRecord};
use mosaic_miden::{MidenTransactionId, Network};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
pub mod desk_store;
use desk_store::{DeskStore, NoteStatus};
pub mod asset_store;
use asset_store::{StoredAsset as RegistryStoredAsset, default_assets};

pub struct RegistryAsset<'a> {
    pub symbol: &'a str,
    pub account: &'a str,
    pub decimals: u8,
    pub max_supply: Option<&'a str>,
    pub owned: bool,
}

#[derive(Debug, Clone)]
pub struct ClientAccountRecord {
    pub account_id: String,
    pub network: String,
    pub account_type: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeskAccountRecord {
    pub account_id: String,
    pub network: Network,
    pub market: Market,
    pub owner_account: String,
    pub market_url: String,
}

#[derive(Debug, Clone)]
pub struct DeskMarketSummary {
    pub desk_account: String,
    pub base_account: String,
    pub quote_account: String,
    pub market_url: String,
    pub owner_account: String,
}

#[derive(Debug, Clone)]
pub struct AccountsForUser {
    pub client_accounts: Vec<ClientAccountRecord>,
    pub desk_accounts: Vec<DeskAccountRecord>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoredOrder {
    pub uuid: String,
    pub order_type: String,
    pub order_json: String,
    pub stage: String,
    pub status: String,
    pub account: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RoleSettings {
    pub is_client: bool,
    pub is_liquidity_provider: bool,
    pub is_desk: bool,
}

/// Desk metadata cached in memory
pub struct DeskMetadata {
    pub client_handle: ClientHandle,
    pub account_id: String,
    pub owner_identifier: String,
    pub network: Network,
    pub market: Market,
    pub owner_account: String,
    pub market_url: String,
}

pub struct Serve {
    store_path: PathBuf,
    desk_store_path: PathBuf,
    clients: Arc<Mutex<HashMap<([u8; 32], Network), ClientHandle>>>,
    desks: Arc<Mutex<HashMap<String, DeskMetadata>>>,
}

impl Serve {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ServeError> {
        let store_path = path.as_ref().to_path_buf();

        if !store_path.exists() {
            return Err(ServeError::PathNotFound(store_path.clone()));
        }

        // Global desk store at the project root (parent of store_path)
        let desk_store_path = store_path.join("mosaic_top.sqlite3");

        Ok(Serve {
            store_path,
            desk_store_path,
            clients: Arc::new(Mutex::new(HashMap::new())),
            desks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Initialize the desk store and restore all desks from the database
    pub async fn init_desks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let desk_store = DeskStore::new(&self.desk_store_path)?;
        let desks = desk_store.list_desks()?;

        tracing::info!(desk_count = desks.len(), "Restoring desks from database");

        for desk in desks {
            let account_id = desk.desk_account.clone();
            let owner_identifier = desk.owner_identifier.clone();
            let network = desk.network;
            let market = desk.market.clone();
            let client_path = desk.path.clone();

            tracing::info!(
                desk_account = %account_id,
                owner = %owner_identifier,
                path = %client_path.display(),
                network = ?network,
                "Restoring desk"
            );

            match ClientHandle::spawn(client_path.clone(), network).await {
                Ok(client_handle) => {
                    let market_url = Self::resolve_market_url(&account_id, desk.market_url.clone());
                    let owner_account = desk.owner_account.clone().unwrap_or_default();

                    let metadata = DeskMetadata {
                        client_handle,
                        account_id: account_id.clone(),
                        owner_identifier: owner_identifier.clone(),
                        network,
                        market,
                        owner_account,
                        market_url,
                    };

                    // Brief lock to insert desk metadata
                    {
                        let mut desks = self.desks.lock().await;
                        desks.insert(account_id.clone(), metadata);
                    }

                    tracing::info!(
                        desk_account = %account_id,
                        owner = %owner_identifier,
                        "Successfully restored desk"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        desk_account = %account_id,
                        path = %client_path.display(),
                        "Failed to spawn client handle"
                    );
                }
            }
        }

        Ok(())
    }

    /// Create a new desk account and register it in the desk store
    pub async fn new_desk_account(
        &self,
        secret: [u8; 32],
        network: Network,
        market: Market,
        owner_account: String,
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        let path = self.client_path(secret, network);
        Self::check_or_create(&path)?;

        let client_handle = self.get_client(secret, network).await?;

        let (_, owner_address) = miden_objects::address::Address::from_bech32(&owner_account)
            .map_err(|e| anyhow::anyhow!("Invalid owner account '{}': {}", owner_account, e))?;
        let owner_account_id = match owner_address {
            miden_objects::address::Address::AccountId(account_id_addr) => account_id_addr.id(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Owner account must resolve to an account id: {}",
                    owner_account
                )
                .into());
            }
        };

        // Create the desk account via the dedicated miden client command
        let (account, remote_market_url) = client_handle
            .create_desk_account(
                market.base.code.clone(),
                market.base.issuer.clone(),
                market.quote.code.clone(),
                market.quote.issuer.clone(),
                owner_account_id,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create desk account: {}", e))?;

        let account_id = account.id();
        let address = miden_objects::address::AccountIdAddress::new(
            account_id,
            miden_objects::address::AddressInterface::Unspecified,
        );

        let network_id = network.to_network_id();
        let account_id_bech32 =
            miden_objects::address::Address::from(address).to_bech32(network_id);

        // Store in account SQLite database
        let store_path = self.store_path(secret, network);
        let store = mosaic_miden::store::Store::new(&store_path)?;
        store.insert_account(&account_id_bech32, network, "Desk", None)?;

        let market_url = Self::resolve_market_url(&account_id_bech32, remote_market_url);

        Self::record_account_order(
            &store,
            &account_id_bech32,
            &AccountOrder::CreateDesk {
                network,
                market: market.clone(),
                owner_account: owner_account.clone(),
            },
        )?;

        // Store in global desk database
        let desk_store = DeskStore::new(&self.desk_store_path)?;
        let owner_identifier = Self::secret_to_string(secret);
        desk_store.insert_desk(
            &account_id_bech32,
            &owner_identifier,
            &owner_account,
            &path,
            network,
            &market,
            &market_url,
        )?;

        // Add to in-memory desk map with metadata
        let metadata = DeskMetadata {
            client_handle: client_handle.clone(),
            account_id: account_id_bech32.clone(),
            owner_identifier,
            network,
            market: market.clone(),
            owner_account: owner_account.clone(),
            market_url: market_url.clone(),
        };

        // Brief lock to insert desk metadata
        {
            let mut desks = self.desks.lock().await;
            desks.insert(account_id_bech32.clone(), metadata);
        }

        tracing::info!(
            account_id = %account_id_bech32,
            network = ?network,
            base = %market.base.code,
            quote = %market.quote.code,
            "Created new desk account"
        );

        Ok((account_id_bech32, market_url))
    }

    /// Get a desk client handle by account identifier
    pub async fn get_desk(&self, account_id: &str) -> Option<ClientHandle> {
        let desks = self.desks.lock().await;
        desks
            .get(account_id)
            .map(|metadata| metadata.client_handle.clone())
    }

    /// List all desks
    pub async fn list_desks(&self) -> Vec<String> {
        let desks = self.desks.lock().await;
        desks.keys().cloned().collect()
    }

    /// Push a note to a desk's note store
    pub async fn desk_push_note(
        &self,
        desk_account: &str,
        note: MosaicNote,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        // Get the desk path from the global desk store
        let desk_store = desk_store::DeskStore::new(&self.desk_store_path)?;
        let stored_desk = desk_store
            .get_desk(desk_account)?
            .ok_or_else(|| anyhow::anyhow!("Desk not found: {}", desk_account))?;
        let desk_path = stored_desk.path;

        // Open the desk's note store
        let desk_note_store_path = desk_path.join("desk_notes.sqlite3");
        let desk_note_store = desk_store::DeskNoteStore::new(&desk_note_store_path)?;

        // Insert the note with 'new' status
        let note_id = desk_note_store.insert_note(&note, NoteStatus::New)?;

        // Attempt to consume immediately using the desk's client handle
        let client_handle = {
            let desks = self.desks.lock().await;
            desks
                .get(desk_account)
                .map(|metadata| metadata.client_handle.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!("Desk client handle not available for {}", desk_account)
                })?
        };

        let (_network_id, address) = miden_objects::address::Address::from_bech32(desk_account)
            .map_err(|e| anyhow::anyhow!("Invalid desk account {}: {}", desk_account, e))?;
        let account_id = match address {
            miden_objects::address::Address::AccountId(addr) => addr.id(),
            _ => {
                return Err(anyhow::anyhow!(
                    "Desk account must resolve to an account id: {}",
                    desk_account
                )
                .into());
            }
        };

        match client_handle
            .consume_note(account_id, note.miden_note.miden_note_hex.clone())
            .await
        {
            Ok(tx_id) => {
                desk_note_store.update_note_status(note_id, NoteStatus::Consumed)?;
                tracing::info!(
                    desk_account = %desk_account,
                    note_id = note_id,
                    tx_id = %tx_id,
                    "Consumed note for desk"
                );
            }
            Err(error) => {
                desk_note_store.update_note_status(note_id, NoteStatus::Invalid)?;
                tracing::error!(
                    desk_account = %desk_account,
                    note_id = note_id,
                    error = %error,
                    "Failed to consume note for desk"
                );
                return Err(anyhow::anyhow!("Failed to consume note: {}", error).into());
            }
        }

        Ok(note_id)
    }

    /// Get all notes from a desk
    pub async fn desk_get_notes(
        &self,
        desk_account: &str,
    ) -> Result<Vec<desk_store::DeskNoteRecord>, Box<dyn std::error::Error>> {
        // Get the desk path from the global desk store
        let desk_store = desk_store::DeskStore::new(&self.desk_store_path)?;
        let stored_desk = desk_store
            .get_desk(desk_account)?
            .ok_or_else(|| anyhow::anyhow!("Desk not found: {}", desk_account))?;
        let desk_path = stored_desk.path;

        // Open the desk's note store
        let desk_note_store_path = desk_path.join("desk_notes.sqlite3");
        let desk_note_store = desk_store::DeskNoteStore::new(&desk_note_store_path)?;

        // Get all notes
        let notes = desk_note_store.get_all_notes()?;

        Ok(notes)
    }

    /// Get desk information including market data from in-memory cache
    pub async fn get_desk_info(
        &self,
        desk_account: &str,
    ) -> Result<(String, Network, Market), Box<dyn std::error::Error>> {
        // Get the desk metadata from in-memory cache
        let desks = self.desks.lock().await;
        let metadata = desks
            .get(desk_account)
            .ok_or_else(|| anyhow::anyhow!("Desk not found: {}", desk_account))?;

        Ok((
            metadata.account_id.clone(),
            metadata.network,
            metadata.market.clone(),
        ))
    }

    /// Get persisted desk metadata suitable for public APIs
    pub async fn get_desk_market_summary(
        &self,
        desk_account: &str,
    ) -> Result<Option<DeskMarketSummary>, Box<dyn std::error::Error>> {
        let desk_store = DeskStore::new(&self.desk_store_path)?;
        let stored = desk_store.get_desk(desk_account)?;

        let Some(stored) = stored else {
            return Ok(None);
        };

        let desks = self.desks.lock().await;

        let owner_account = stored
            .owner_account
            .clone()
            .filter(|value| !value.is_empty())
            .or_else(|| {
                desks
                    .get(desk_account)
                    .map(|meta| meta.owner_account.clone())
            })
            .unwrap_or_default();

        let market_url = {
            let provided = stored.market_url.clone().or_else(|| {
                desks
                    .get(desk_account)
                    .map(|meta| meta.market_url.clone())
            });
            Self::resolve_market_url(desk_account, provided)
        };

        let summary = DeskMarketSummary {
            desk_account: desk_account.to_string(),
            base_account: stored.market.base.issuer.clone(),
            quote_account: stored.market.quote.issuer.clone(),
            market_url,
            owner_account,
        };

        Ok(Some(summary))
    }

    fn resolve_market_url(account_id: &str, provided: Option<String>) -> String {
        provided
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| Self::default_market_url(account_id))
    }

    fn default_market_url(account_id: &str) -> String {
        std::env::var("MOSAIC_SERVER")
            .map(|base| format!("{}/desk/{}", base.trim_end_matches('/'), account_id))
            .unwrap_or_else(|_| format!("/desk/{}", account_id))
    }

    fn secret_to_string(secret: [u8; 32]) -> String {
        bs58::encode(secret).into_string()
    }

    fn network_from_account(account: &str) -> Result<Network, anyhow::Error> {
        let (network_id, _) = miden_objects::address::Address::from_bech32(account)
            .map_err(|e| anyhow::anyhow!("Invalid account '{}': {}", account, e))?;

        Network::from_network_id(network_id).ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported network '{}' for account {}",
                network_id,
                account
            )
        })
    }

    fn order_metadata(order: &mosaic_fi::note::Order) -> (String, Option<String>) {
        use mosaic_fi::note::Order::*;

        match order {
            KYCPassed { .. } => ("KYCPassed".to_string(), None),
            QuoteRequestOffer { uuid, .. } => {
                ("QuoteRequestOffer".to_string(), Some(uuid.to_string()))
            }
            QuoteRequestNoOffer { uuid, .. } => {
                ("QuoteRequestNoOffer".to_string(), Some(uuid.to_string()))
            }
            QuoteRequest { uuid, .. } => ("QuoteRequest".to_string(), Some(uuid.to_string())),
            LimitOrder { uuid, .. } => ("LimitOrder".to_string(), Some(uuid.to_string())),
            LiquidityOffer { uuid, .. } => ("LiquidityOffer".to_string(), Some(uuid.to_string())),
            FundAccount { .. } => ("FundAccount".to_string(), None),
            LimitBuyOrderLocked => ("LimitBuyOrderLocked".to_string(), None),
            LimitBuyOrderNotLocked => ("LimitBuyOrderNotLocked".to_string(), None),
            LimitSellOrderLocked => ("LimitSellOrderLocked".to_string(), None),
            LimitSellOrderNotLocked => ("LimitSellOrderNotLocked".to_string(), None),
        }
    }

    fn record_account_order(
        store: &mosaic_miden::store::Store,
        account_id: &str,
        order: &AccountOrder,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let order_record = OrderRecord {
            uuid: Uuid::new_v4().to_string(),
            order_type: order.kind().to_string(),
            order_json: serde_json::to_string(order)?,
            stage: "create".to_string(),
            status: "succeeded".to_string(),
            account: account_id.to_string(),
            created_at: None,
        };

        store.upsert_order(&order_record)?;

        Ok(())
    }

    fn client_path(&self, secret: [u8; 32], network: Network) -> PathBuf {
        let secret_str = Self::secret_to_string(secret);
        let network_prefix = match network {
            Network::Testnet => "testnet",
            Network::Localnet => "localnet",
        };
        let dir_name = format!("{}_{}", network_prefix, secret_str);
        self.store_path.join(dir_name)
    }

    fn store_path(&self, secret: [u8; 32], network: Network) -> PathBuf {
        self.client_path(secret, network).join("mosaic.sqlite3")
    }

    fn check_or_create(path: &PathBuf) -> Result<bool, anyhow::Error> {
        if path.exists() {
            Ok(false)
        } else {
            std::fs::create_dir_all(path)?;
            Ok(true)
        }
    }

    pub async fn list_accounts(
        &self,
        secret: [u8; 32],
    ) -> Result<AccountsForUser, Box<dyn std::error::Error>> {
        let mut client_accounts = Vec::new();

        // Collect client-managed accounts from each network store
        for network in [Network::Testnet, Network::Localnet] {
            let store_path = self.store_path(secret, network);

            if !store_path.exists() {
                continue;
            }

            let store = mosaic_miden::store::Store::new(&store_path)?;
            let accounts = store.list_accounts()?;

            for (account_id, network_str, account_type, name) in accounts {
                if account_type == "Desk" {
                    // Desk accounts are surfaced via the desk store for richer metadata
                    continue;
                }

                client_accounts.push(ClientAccountRecord {
                    account_id,
                    network: network_str,
                    account_type,
                    name,
                });
            }
        }

        // Collect desk accounts owned by this user
        let owner_identifier = Self::secret_to_string(secret);
        let desk_store = DeskStore::new(&self.desk_store_path)?;
        let desks_from_db = desk_store.list_desks_for_owner(&owner_identifier)?;

        let desks = self.desks.lock().await;
        let desk_accounts = desks_from_db
            .into_iter()
            .map(|desk| {
                let owner_account = desk
                    .owner_account
                    .clone()
                    .filter(|value| !value.is_empty())
                    .or_else(|| {
                        desks
                            .get(&desk.desk_account)
                            .map(|meta| meta.owner_account.clone())
                    })
                    .unwrap_or_default();
                let market_url = Self::resolve_market_url(
                    &desk.desk_account,
                    desk.market_url.clone().or_else(|| {
                        desks
                            .get(&desk.desk_account)
                            .map(|meta| meta.market_url.clone())
                    }),
                );

                DeskAccountRecord {
                    account_id: desk.desk_account,
                    network: desk.network,
                    market: desk.market,
                    owner_account,
                    market_url,
                }
            })
            .collect();

        Ok(AccountsForUser {
            client_accounts,
            desk_accounts,
        })
    }

    pub fn register_asset(
        &self,
        secret: [u8; 32],
        asset: RegistryAsset<'_>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let network = Self::network_from_account(asset.account)?;
        let client_dir = self.client_path(secret, network);
        Self::check_or_create(&client_dir)?;

        let store_path = client_dir.join("mosaic.sqlite3");
        let store = mosaic_miden::store::Store::new(&store_path)?;

        let record = AssetRecord {
            symbol: asset.symbol.to_string(),
            account: asset.account.to_string(),
            decimals: asset.decimals,
            max_supply: asset.max_supply.map(|value| value.to_string()),
            owned: asset.owned,
        };

        store.upsert_asset(&record)?;
        Ok(())
    }

    pub fn list_assets_for_user(
        &self,
        secret: [u8; 32],
    ) -> Result<Vec<RegistryStoredAsset>, Box<dyn std::error::Error>> {
        let mut assets_map: HashMap<String, RegistryStoredAsset> = HashMap::new();

        for asset in default_assets() {
            let key = format!("{}::{}", asset.account, asset.symbol);
            assets_map.insert(key, asset);
        }

        for network in [Network::Testnet, Network::Localnet] {
            let client_dir = self.client_path(secret, network);
            if !client_dir.exists() {
                continue;
            }

            let store_path = client_dir.join("mosaic.sqlite3");
            let store = mosaic_miden::store::Store::new(&store_path)?;

            for asset in store.list_assets()? {
                let max_supply = asset.max_supply.clone().unwrap_or_else(|| "0".to_string());

                let key = format!("{}::{}", asset.account, asset.symbol);
                assets_map.insert(
                    key,
                    RegistryStoredAsset {
                        symbol: asset.symbol,
                        account: asset.account,
                        max_supply,
                        decimals: asset.decimals,
                        verified: asset.owned,
                        owner: asset.owned,
                        hidden: false,
                    },
                );
            }
        }

        Ok(assets_map.into_values().collect())
    }

    pub fn list_orders_for_user(
        &self,
        secret: [u8; 32],
    ) -> Result<Vec<StoredOrder>, Box<dyn std::error::Error>> {
        let mut orders = Vec::new();

        for network in [Network::Testnet, Network::Localnet] {
            let client_dir = self.client_path(secret, network);
            if !client_dir.exists() {
                continue;
            }

            let store_path = client_dir.join("mosaic.sqlite3");
            let store = mosaic_miden::store::Store::new(&store_path)?;

            for order in store.list_orders()? {
                orders.push(StoredOrder {
                    uuid: order.uuid,
                    order_type: order.order_type,
                    order_json: order.order_json,
                    stage: order.stage,
                    status: order.status,
                    account: order.account,
                    created_at: order.created_at,
                });
            }
        }

        orders.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(orders)
    }

    pub fn get_role_settings(
        &self,
        secret: [u8; 32],
    ) -> Result<RoleSettings, Box<dyn std::error::Error>> {
        let client_dir = self.client_path(secret, Network::Testnet);
        Self::check_or_create(&client_dir)?;
        let store_path = client_dir.join("mosaic.sqlite3");
        let store = match mosaic_miden::store::Store::new(&store_path) {
            Ok(store) => store,
            Err(_) => {
                Self::check_or_create(&client_dir)?;
                mosaic_miden::store::Store::new(&store_path)?
            }
        };

        let settings = store.get_settings()?;
        Ok(RoleSettings {
            is_client: settings.is_client,
            is_liquidity_provider: settings.is_liquidity_provider,
            is_desk: settings.is_desk,
        })
    }

    pub fn update_role_settings(
        &self,
        secret: [u8; 32],
        settings: RoleSettings,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client_dir = self.client_path(secret, Network::Testnet);
        Self::check_or_create(&client_dir)?;
        let store_path = client_dir.join("mosaic.sqlite3");
        let store = match mosaic_miden::store::Store::new(&store_path) {
            Ok(store) => store,
            Err(_) => {
                Self::check_or_create(&client_dir)
                    .map_err(|e| anyhow::anyhow!("Failed to initialise client directory: {}", e))?;
                mosaic_miden::store::Store::new(&store_path)?
            }
        };

        let record = SettingsRecord {
            is_client: settings.is_client,
            is_liquidity_provider: settings.is_liquidity_provider,
            is_desk: settings.is_desk,
        };
        store.update_settings(&record)?;
        Ok(())
    }

    pub fn list_default_assets(&self) -> Vec<RegistryStoredAsset> {
        default_assets()
    }

    pub async fn get_client(
        &self,
        secret: [u8; 32],
        network: Network,
    ) -> Result<ClientHandle, Box<dyn std::error::Error>> {
        // Brief lock to check cache
        {
            let clients = self.clients.lock().await;
            if let Some(client_handle) = clients.get(&(secret, network)) {
                return Ok(client_handle.clone());
            }
        }
        // Lock released before spawning

        let path = self.client_path(secret, network);

        // Spawn client without holding lock (this is the slow operation)
        let client_handle = ClientHandle::spawn(path, network).await?;

        // Brief lock to insert into cache
        {
            let mut clients = self.clients.lock().await;
            // Check again in case another request spawned it while we were spawning
            if let Some(existing_handle) = clients.get(&(secret, network)) {
                return Ok(existing_handle.clone());
            }
            clients.insert((secret, network), client_handle.clone());
        }

        Ok(client_handle)
    }

    pub async fn new_account(
        &self,
        secret: [u8; 32],
        account_type: AccountType,
        network: Network,
        name: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let path = self.client_path(secret, network);
        Self::check_or_create(&path)?;

        let client_handle = self.get_client(secret, network).await?;

        let account = client_handle
            .create_account()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create account: {}", e))?;

        let account_id = account.id();
        let address = miden_objects::address::AccountIdAddress::new(
            account_id,
            miden_objects::address::AddressInterface::Unspecified,
        );

        let network_id = network.to_network_id();
        let account_id_bech32 =
            miden_objects::address::Address::from(address).to_bech32(network_id);

        // Store in SQLite database
        let store_path = self.store_path(secret, network);
        let store = mosaic_miden::store::Store::new(&store_path)?;

        let account_type_str = match account_type {
            AccountType::Client => "Client",
            AccountType::Desk => "Desk",
            AccountType::Liquidity => "Liquidity",
            AccountType::Faucet => "Faucet",
        };

        let account_name = name.map(|value| value.to_string());

        store.insert_account(
            &account_id_bech32,
            network,
            account_type_str,
            account_name.as_deref(),
        )?;

        match account_type {
            AccountType::Client => {
                let order = AccountOrder::CreateClient {
                    network,
                    name: account_name.clone(),
                };
                Self::record_account_order(&store, &account_id_bech32, &order)?;
            }
            AccountType::Liquidity => {
                let order = AccountOrder::CreateLiquidity { network };
                Self::record_account_order(&store, &account_id_bech32, &order)?;
            }
            _ => {}
        }

        Ok(account_id_bech32)
    }

    pub async fn new_faucet_account(
        &self,
        secret: [u8; 32],
        network: Network,
        token_symbol: String,
        decimals: u8,
        max_supply: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let path = self.client_path(secret, network);
        Self::check_or_create(&path)?;

        let client_handle = self.get_client(secret, network).await?;

        let account = client_handle
            .create_faucet_account(token_symbol.clone(), decimals, max_supply)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create faucet account: {}", e))?;

        let account_id = account.id();
        let address = miden_objects::address::AccountIdAddress::new(
            account_id,
            miden_objects::address::AddressInterface::Unspecified,
        );
        let network_id = network.to_network_id();
        let account_id_bech32 =
            miden_objects::address::Address::from(address).to_bech32(network_id);

        // Store in SQLite database
        let store_path = self.store_path(secret, network);
        let store = mosaic_miden::store::Store::new(&store_path)?;

        store.insert_account(
            &account_id_bech32,
            network,
            "Faucet",
            Some(token_symbol.as_str()),
        )?;

        let order = AccountOrder::CreateFaucet {
            network,
            token_symbol: token_symbol.clone(),
            decimals,
            max_supply,
        };
        Self::record_account_order(&store, &account_id_bech32, &order)?;

        Ok(account_id_bech32)
    }

    pub async fn create_account_order(
        &self,
        secret: [u8; 32],
        order: AccountOrder,
    ) -> Result<AccountOrderResult, Box<dyn std::error::Error>> {
        match order {
            AccountOrder::CreateClient { network, name } => {
                let normalized_name = name
                    .as_ref()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(|value| value.to_string());

                let account_id = self
                    .new_account(
                        secret,
                        AccountType::Client,
                        network,
                        normalized_name.as_deref(),
                    )
                    .await?;

                Ok(AccountOrderResult::Client {
                    account_id,
                    name: normalized_name,
                })
            }
            AccountOrder::CreateDesk {
                network,
                market,
                owner_account,
            } => {
                let (account_id, market_url) = self
                    .new_desk_account(secret, network, market.clone(), owner_account.clone())
                    .await?;

                Ok(AccountOrderResult::Desk {
                    account_id,
                    market,
                    owner_account,
                    market_url,
                })
            }
            AccountOrder::ActivateDesk {
                desk_account,
                owner_account,
            } => {
                self.activate_desk_account(secret, &desk_account, &owner_account)
                    .await?;

                Ok(AccountOrderResult::DeskActivated {
                    desk_account,
                    owner_account,
                })
            }
            AccountOrder::DeactivateDesk {
                desk_account,
                owner_account,
            } => {
                self.deactivate_desk_account(secret, &desk_account, &owner_account)
                    .await?;

                Ok(AccountOrderResult::DeskDeactivated {
                    desk_account,
                    owner_account,
                })
            }
            AccountOrder::CreateFaucet {
                network,
                token_symbol,
                decimals,
                max_supply,
            } => {
                let account_id = self
                    .new_faucet_account(secret, network, token_symbol.clone(), decimals, max_supply)
                    .await?;

                let max_supply_string = max_supply.to_string();
                let asset = RegistryAsset {
                    symbol: token_symbol.as_str(),
                    account: account_id.as_str(),
                    decimals,
                    max_supply: Some(max_supply_string.as_str()),
                    owned: true,
                };

                if let Err(err) = self.register_asset(secret, asset) {
                    tracing::warn!(
                        error = %err,
                        account_id = %account_id,
                        token_symbol = %token_symbol,
                        "Failed to register faucet asset for user"
                    );
                }

                Ok(AccountOrderResult::Faucet {
                    account_id,
                    token_symbol,
                    decimals,
                    max_supply,
                })
            }
            AccountOrder::CreateLiquidity { network } => {
                let account_id = self
                    .new_account(secret, AccountType::Liquidity, network, None)
                    .await?;

                Ok(AccountOrderResult::Liquidity { account_id })
            }
        }
    }

    async fn activate_desk_account(
        &self,
        _secret: [u8; 32],
        _desk_account: &str,
        _owner_account: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        todo!("create note, commit and consume");
    }

    async fn deactivate_desk_account(
        &self,
        _secret: [u8; 32],
        _desk_account: &str,
        _owner_account: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        todo!("create note, commit and consume");
    }

    pub async fn create_private_note(
        &self,
        secret: [u8; 32],
        network: Network,
        account_id_bech32: String,
        order: mosaic_fi::note::Order,
        commit: bool,
    ) -> Result<MosaicNote, Box<dyn std::error::Error>> {
        let client_handle = self.get_client(secret, network).await?;

        let (_network_id, address) =
            miden_objects::address::Address::from_bech32(&account_id_bech32)?;
        let account_id = match address {
            miden_objects::address::Address::AccountId(account_id_addr) => account_id_addr.id(),
            _ => {
                return Err(
                    format!("Invalid address type for account ID: {}", account_id_bech32).into(),
                );
            }
        };

        let store_path = self.store_path(secret, network);
        let store = mosaic_miden::store::Store::new(&store_path)?;

        let _account_record = client_handle
            .get_account(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id_bech32))?;

        let order_clone = order.clone();
        let (order_type, uuid_opt) = Self::order_metadata(&order_clone);
        let generated_uuid = uuid_opt.unwrap_or_else(|| Uuid::new_v4().to_string());
        let order_json = serde_json::to_string(&order_clone)?;

        let mut order_record = OrderRecord {
            uuid: generated_uuid.clone(),
            order_type,
            order_json,
            stage: "create".to_string(),
            status: String::new(),
            account: account_id_bech32.clone(),
            created_at: None,
        };

        let mut mosaic_note = match mosaic_fi::note::compile_note_from_account_id(account_id, order)
        {
            Ok(note) => note,
            Err(err) => {
                order_record.status = "failed".to_string();
                let _ = store.upsert_order(&order_record);
                return Err(err);
            }
        };

        if commit {
            match client_handle
                .commit_note(account_id, mosaic_note.miden_note.miden_note_hex.clone())
                .await
            {
                Ok(tx_commit_id) => {
                    mosaic_note.status = MosaicNoteStatus::Committed(tx_commit_id);
                }
                Err(e) => {
                    order_record.status = "failed".to_string();
                    let _ = store.upsert_order(&order_record);
                    return Err(anyhow::anyhow!("Failed to commit note: {}", e).into());
                }
            }
        }

        order_record.status = if commit {
            match mosaic_note.status {
                MosaicNoteStatus::Committed(_) => "committed".to_string(),
                _ => "created".to_string(),
            }
        } else {
            "created".to_string()
        };

        store.upsert_order(&order_record)?;

        Ok(mosaic_note)
    }

    pub async fn create_note_from_masm(
        &self,
        secret: [u8; 32],
        network: Network,
        account_id_bech32: String,
        note_type: mosaic_miden::note::NoteType,
        program: String,
        libraries: Vec<(String, String)>,
        inputs: Vec<(String, mosaic_miden::note::Value)>,
        note_secret: Option<[u64; 4]>,
    ) -> Result<mosaic_miden::note::MidenNote, Box<dyn std::error::Error>> {
        let client_handle = self.get_client(secret, network).await?;

        let (_network_id, address) =
            miden_objects::address::Address::from_bech32(&account_id_bech32)?;
        let account_id = match address {
            miden_objects::address::Address::AccountId(account_id_addr) => account_id_addr.id(),
            _ => {
                return Err(
                    format!("Invalid address type for account ID: {}", account_id_bech32).into(),
                );
            }
        };

        let _account_record = client_handle
            .get_account(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id_bech32))?;

        // Create MidenAbstractNote
        let abstract_note = mosaic_miden::note::MidenAbstractNote {
            version: mosaic_miden::version::VERSION_STRING.to_string(),
            note_type,
            program,
            libraries,
        };

        // Use default note_secret if not provided
        let secret_word = if let Some(note_secret_arr) = note_secret {
            // Convert [u64; 4] to Word (which is [Felt; 4])
            use miden_objects::Felt;
            [
                Felt::new(note_secret_arr[0]),
                Felt::new(note_secret_arr[1]),
                Felt::new(note_secret_arr[2]),
                Felt::new(note_secret_arr[3]),
            ]
        } else {
            *miden_objects::Word::default()
        };

        // Compile the note
        let miden_note = mosaic_miden::note::compile_note(
            abstract_note,
            account_id,
            secret_word.into(),
            inputs,
        )?;

        // Commit the note
        let _tx_commit_id = client_handle
            .commit_note(account_id, miden_note.miden_note_hex.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to commit note: {}", e))?;

        Ok(miden_note)
    }

    pub async fn consume_note(
        &self,
        secret: [u8; 32],
        network: Network,
        account_id_bech32: String,
        miden_note: mosaic_miden::note::MidenNote,
    ) -> Result<MidenTransactionId, Box<dyn std::error::Error>> {
        tracing::info!(
            account_id_bech32 = %account_id_bech32,
            network = ?network,
            note_version = %miden_note.version,
            note_type = ?miden_note.note_type,
            note_hex_length = miden_note.miden_note_hex.len(),
            "Serve layer: Starting note consumption"
        );

        let client_handle = self.get_client(secret, network).await?;

        let (_network_id, address) =
            miden_objects::address::Address::from_bech32(&account_id_bech32).map_err(|e| {
                tracing::error!(
                    error = %e,
                    account_id_bech32 = %account_id_bech32,
                    "Failed to parse bech32 address"
                );
                e
            })?;
        let account_id = match address {
            miden_objects::address::Address::AccountId(account_id_addr) => account_id_addr.id(),
            _ => {
                tracing::error!(
                    account_id_bech32 = %account_id_bech32,
                    address_type = ?address,
                    "Invalid address type for account ID"
                );
                return Err(
                    format!("Invalid address type for account ID: {}", account_id_bech32).into(),
                );
            }
        };

        tracing::info!(
            account_id = %account_id,
            account_id_bech32 = %account_id_bech32,
            "Serve layer: Parsed account ID"
        );

        let account_record = client_handle
            .get_account(account_id)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    account_id = %account_id,
                    "Failed to get account from client"
                );
                anyhow::anyhow!("Failed to get account: {}", e)
            })?
            .ok_or_else(|| {
                tracing::error!(
                    account_id = %account_id,
                    account_id_bech32 = %account_id_bech32,
                    "Account not found in client store"
                );
                anyhow::anyhow!("Account not found: {}", account_id_bech32)
            })?;

        tracing::info!(
            account_id = %account_id,
            account_status = ?account_record.status(),
            "Serve layer: Retrieved account record"
        );

        // Consume the note
        let transaction_id = client_handle
            .consume_note(account_id, miden_note.miden_note_hex.clone())
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    account_id = %account_id,
                    note_hex_length = miden_note.miden_note_hex.len(),
                    "Serve layer: Failed to consume note"
                );
                anyhow::anyhow!("Failed to consume note: {}", e)
            })?;

        tracing::info!(
            transaction_id = %transaction_id,
            account_id = %account_id,
            "Serve layer: Successfully consumed note"
        );

        Ok(transaction_id)
    }

    pub async fn get_account_status(
        &self,
        secret: [u8; 32],
        network: Network,
        account_id_bech32: String,
    ) -> Result<mosaic_miden::AccountStatusData, Box<dyn std::error::Error>> {
        let client_handle = self.get_client(secret, network).await?;

        let (_network_id, address) =
            miden_objects::address::Address::from_bech32(&account_id_bech32)?;
        let account_id = match address {
            miden_objects::address::Address::AccountId(account_id_addr) => account_id_addr.id(),
            _ => {
                return Err(
                    format!("Invalid address type for account ID: {}", account_id_bech32).into(),
                );
            }
        };

        // Get account type from the store
        let store_path = self.store_path(secret, network);
        let store = mosaic_miden::store::Store::new(&store_path)?;
        let all_accounts = store.list_accounts()?;

        let account_type = all_accounts
            .iter()
            .find(|(id, _, _, _)| id == &account_id_bech32)
            .map(|(_, _, typ, _)| typ.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Get account status from client
        let mut account_status = client_handle
            .get_account_status(account_id, network)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account status: {}", e))?;

        // Update with the actual account type from the store
        account_status.account_type = account_type;

        Ok(account_status)
    }

    /// Flush all cached clients
    /// Returns the number of clients that were flushed
    pub async fn flush(&self) -> usize {
        let mut clients = self.clients.lock().await;
        let count = clients.len();
        clients.clear();
        count
    }

    /// Flush cached clients for a specific user secret
    pub async fn flush_clients_for_secret(&self, secret: [u8; 32]) -> usize {
        let mut clients = self.clients.lock().await;
        let before = clients.len();
        clients.retain(|(entry_secret, _), _| entry_secret != &secret);
        before - clients.len()
    }
}

#[derive(Debug)]
pub enum ServeError {
    PathNotFound(PathBuf),
    InvalidPath(String),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServeError::PathNotFound(path) => {
                write!(f, "Store path does not exist: {}", path.display())
            }
            ServeError::InvalidPath(msg) => {
                write!(f, "Invalid path: {}", msg)
            }
        }
    }
}

impl std::error::Error for ServeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_account_order_persists_entry() {
        let store = mosaic_miden::store::Store::new(":memory:").expect("store");
        store
            .insert_account("test_account", Network::Testnet, "Client", None)
            .expect("insert account");

        let order = AccountOrder::CreateClient {
            network: Network::Testnet,
            name: Some("Primary".to_string()),
        };

        Serve::record_account_order(&store, "test_account", &order).expect("order recorded");

        let orders = store.list_orders().expect("list orders");
        assert_eq!(orders.len(), 1);
        let order = &orders[0];
        assert_eq!(order.order_type, "CreateClientAccount");
        assert_eq!(order.stage, "create");
        assert_eq!(order.status, "succeeded");
        assert_eq!(order.account, "test_account");
    }
}
