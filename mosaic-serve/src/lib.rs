use mosaic_fi::AccountType;
use mosaic_fi::note::{Market, MosaicNote, MosaicNoteStatus};
use mosaic_miden::client::ClientHandle;
use mosaic_miden::{MidenTransactionId, Network};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Serve {
    store_path: PathBuf,
    clients: HashMap<([u8; 32], Network), ClientHandle>,
}

impl Serve {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ServeError> {
        let store_path = path.as_ref().to_path_buf();

        if !store_path.exists() {
            return Err(ServeError::PathNotFound(store_path));
        }

        Ok(Serve {
            store_path,
            clients: HashMap::new(),
        })
    }

    fn secret_to_string(secret: [u8; 32]) -> String {
        bs58::encode(secret).into_string()
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
    ) -> Result<Vec<(String, String, String)>, Box<dyn std::error::Error>> {
        let mut all_accounts = Vec::new();

        // Check both testnet and localnet directories
        for network in [Network::Testnet, Network::Localnet] {
            let store_path = self.store_path(secret, network);

            if !store_path.exists() {
                continue;
            }

            let store = mosaic_miden::store::Store::new(&store_path)?;
            let accounts = store.list_accounts()?;

            for (account_id, network_str, account_type) in accounts {
                all_accounts.push((account_id, network_str, account_type));
            }
        }

        Ok(all_accounts)
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

        store.insert_account(&account_id_bech32, network, account_type_str)?;

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
            .create_faucet_account(token_symbol, decimals, max_supply)
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

        store.insert_account(&account_id_bech32, network, "Faucet")?;

        Ok(account_id_bech32)
    }

    pub async fn create_private_note(
        &mut self,
        secret: [u8; 32],
        network: Network,
        account_id_bech32: String,
        order: mosaic_fi::note::Order,
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

        let _account_record = client_handle
            .get_account(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id_bech32))?;

        let mut mosaic_note = mosaic_fi::note::compile_note_from_account_id(account_id, order)?;

        let tx_commit_id = client_handle
            .commit_note(account_id, mosaic_note.miden_note.miden_note_hex.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to commit note: {}", e))?;

        mosaic_note.status = MosaicNoteStatus::Committed(tx_commit_id);

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
            .find(|(id, _, _)| id == &account_id_bech32)
            .map(|(_, _, typ)| typ.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Get account status from client
        let mut account_status = client_handle
            .get_account_status(account_id)
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
}

#[derive(Debug)]
pub enum ServeError {
    PathNotFound(PathBuf),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServeError::PathNotFound(path) => {
                write!(f, "Store path does not exist: {}", path.display())
            }
        }
    }
}

impl std::error::Error for ServeError {}

pub async fn post_note(_market: Market, _note: MosaicNote) -> Result<(), ()> {
    Ok(())
}

pub async fn get_notes(_market: Market) -> Result<Vec<MosaicNote>, ()> {
    Ok(vec![])
}
