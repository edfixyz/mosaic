use mosaic_fi::note::{MosaicNote, MosaicNoteStatus};
use mosaic_fi::{AccountType, Market};
use mosaic_miden::client::ClientHandle;
use mosaic_miden::store::{AssetRecord, OrderRecord, SettingsRecord};
use mosaic_miden::{MidenTransactionId, Network};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub mod desk_store;
use desk_store::DeskStore;
pub mod asset_store;
use asset_store::{StoredAsset as RegistryStoredAsset, default_assets};

pub struct RegistryAsset<'a> {
    pub symbol: &'a str,
    pub account: &'a str,
    pub decimals: u8,
    pub max_supply: Option<&'a str>,
    pub owned: bool,
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
    pub network: Network,
    pub market: Market,
}

pub struct Serve {
    store_path: PathBuf,
    desk_store_path: PathBuf,
    clients: HashMap<([u8; 32], Network), ClientHandle>,
    desks: HashMap<Uuid, DeskMetadata>,
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
            clients: HashMap::new(),
            desks: HashMap::new(),
        })
    }

    /// Initialize the desk store and restore all desks from the database
    pub async fn init_desks(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let desk_store = DeskStore::new(&self.desk_store_path)?;
        let desks = desk_store.list_desks()?;

        tracing::info!(desk_count = desks.len(), "Restoring desks from database");

        for (uuid, path, network, market) in desks {
            tracing::info!(
                uuid = %uuid,
                path = %path.display(),
                network = ?network,
                "Restoring desk"
            );

            match ClientHandle::spawn(path.clone(), network).await {
                Ok(client_handle) => {
                    // Get the desk's account ID
                    let store_path = path.join("mosaic.sqlite3");
                    match mosaic_miden::store::Store::new(&store_path) {
                        Ok(store) => {
                            match store.list_accounts() {
                                Ok(accounts) => {
                                    // Find the Desk account
                                    if let Some((account_id, _, _, _)) = accounts
                                        .iter()
                                        .find(|(_, _, acc_type, _)| acc_type == "Desk")
                                    {
                                        let metadata = DeskMetadata {
                                            client_handle,
                                            account_id: account_id.clone(),
                                            network,
                                            market,
                                        };
                                        self.desks.insert(uuid, metadata);
                                        tracing::info!(
                                            uuid = %uuid,
                                            account_id = %account_id,
                                            "Successfully restored desk"
                                        );
                                    } else {
                                        tracing::error!(
                                            uuid = %uuid,
                                            path = %path.display(),
                                            "Desk account not found in store"
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        uuid = %uuid,
                                        path = %path.display(),
                                        "Failed to list accounts"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                uuid = %uuid,
                                path = %path.display(),
                                "Failed to open store"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        uuid = %uuid,
                        path = %path.display(),
                        "Failed to spawn client handle"
                    );
                }
            }
        }

        Ok(())
    }

    /// Create a new desk account and register it in the desk store
    pub async fn new_desk_account(
        &mut self,
        secret: [u8; 32],
        network: Network,
        market: Market,
    ) -> Result<(Uuid, String), Box<dyn std::error::Error>> {
        let uuid = Uuid::new_v4();
        let path = self.client_path(secret, network);
        Self::check_or_create(&path)?;

        let client_handle = self.get_client(secret, network).await?;

        // Create the account
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

        // Store in account SQLite database
        let store_path = self.store_path(secret, network);
        let store = mosaic_miden::store::Store::new(&store_path)?;
        store.insert_account(&account_id_bech32, network, "Desk", None)?;

        // Store in global desk database
        let desk_store = DeskStore::new(&self.desk_store_path)?;
        desk_store.insert_desk(uuid, &path, network, &market)?;

        // Add to in-memory desk map with metadata
        let metadata = DeskMetadata {
            client_handle: client_handle.clone(),
            account_id: account_id_bech32.clone(),
            network,
            market: market.clone(),
        };
        self.desks.insert(uuid, metadata);

        tracing::info!(
            uuid = %uuid,
            account_id = %account_id_bech32,
            network = ?network,
            base = %market.base.code,
            quote = %market.quote.code,
            "Created new desk account"
        );

        Ok((uuid, account_id_bech32))
    }

    /// Get a desk client handle by UUID
    pub fn get_desk(&self, uuid: Uuid) -> Option<&ClientHandle> {
        self.desks
            .get(&uuid)
            .map(|metadata| &metadata.client_handle)
    }

    /// List all desks
    pub fn list_desks(&self) -> Vec<Uuid> {
        self.desks.keys().copied().collect()
    }

    /// Push a note to a desk's note store
    pub async fn desk_push_note(
        &self,
        desk_uuid: Uuid,
        note: MosaicNote,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        // Get the desk path from the global desk store
        let desk_store = desk_store::DeskStore::new(&self.desk_store_path)?;
        let (desk_path, _network, _market) = desk_store
            .get_desk(desk_uuid)?
            .ok_or_else(|| anyhow::anyhow!("Desk not found: {}", desk_uuid))?;

        // Open the desk's note store
        let desk_note_store_path = desk_path.join("desk_notes.sqlite3");
        let desk_note_store = desk_store::DeskNoteStore::new(&desk_note_store_path)?;

        // Insert the note with 'new' status
        let note_id = desk_note_store.insert_note(&note, desk_store::NoteStatus::New)?;

        tracing::info!(
            desk_uuid = %desk_uuid,
            note_id = note_id,
            "Pushed note to desk"
        );

        Ok(note_id)
    }

    /// Get all notes from a desk
    pub async fn desk_get_notes(
        &self,
        desk_uuid: Uuid,
    ) -> Result<Vec<desk_store::DeskNoteRecord>, Box<dyn std::error::Error>> {
        // Get the desk path from the global desk store
        let desk_store = desk_store::DeskStore::new(&self.desk_store_path)?;
        let (desk_path, _network, _market) = desk_store
            .get_desk(desk_uuid)?
            .ok_or_else(|| anyhow::anyhow!("Desk not found: {}", desk_uuid))?;

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
        desk_uuid: Uuid,
    ) -> Result<(String, Network, Market), Box<dyn std::error::Error>> {
        // Get the desk metadata from in-memory cache
        let metadata = self
            .desks
            .get(&desk_uuid)
            .ok_or_else(|| anyhow::anyhow!("Desk not found: {}", desk_uuid))?;

        Ok((
            metadata.account_id.clone(),
            metadata.network,
            metadata.market.clone(),
        ))
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
    ) -> Result<Vec<(String, String, String, Option<String>)>, Box<dyn std::error::Error>> {
        let mut all_accounts = Vec::new();

        // Check both testnet and localnet directories
        for network in [Network::Testnet, Network::Localnet] {
            let store_path = self.store_path(secret, network);

            if !store_path.exists() {
                continue;
            }

            let store = mosaic_miden::store::Store::new(&store_path)?;
            let accounts = store.list_accounts()?;

            for (account_id, network_str, account_type, name) in accounts {
                all_accounts.push((account_id, network_str, account_type, name));
            }
        }

        Ok(all_accounts)
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
        let store_path = client_dir.join("mosaic.sqlite3");
        let store = mosaic_miden::store::Store::new(&store_path)?;

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
        &mut self,
        secret: [u8; 32],
        network: Network,
    ) -> Result<ClientHandle, Box<dyn std::error::Error>> {
        if let Some(client_handle) = self.clients.get(&(secret, network)) {
            return Ok(client_handle.clone());
        }

        let path = self.client_path(secret, network);

        let client_handle = ClientHandle::spawn(path, network).await?;

        self.clients
            .insert((secret, network), client_handle.clone());

        Ok(client_handle)
    }

    pub async fn new_account(
        &mut self,
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

        store.insert_account(&account_id_bech32, network, account_type_str, name)?;

        Ok(account_id_bech32)
    }

    pub async fn new_faucet_account(
        &mut self,
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

        Ok(account_id_bech32)
    }

    pub async fn create_private_note(
        &mut self,
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
        &mut self,
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
        &mut self,
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
        &mut self,
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
    pub fn flush(&mut self) -> usize {
        let count = self.clients.len();
        self.clients.clear();
        count
    }

    /// Flush cached clients for a specific user secret
    pub fn flush_clients_for_secret(&mut self, secret: [u8; 32]) -> usize {
        let before = self.clients.len();
        self.clients
            .retain(|(entry_secret, _), _| entry_secret != &secret);
        before - self.clients.len()
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
