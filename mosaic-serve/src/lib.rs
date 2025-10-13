use mosaic_fi::AccountType;
use mosaic_fi::note::{Market, MosaicNote};
use mosaic_miden::Network;
use mosaic_miden::client::ClientHandle;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Serve {
    store_path: PathBuf,
    // Client cache: (identifier, network) -> ClientHandle
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

    fn identifier_to_string(identifier: [u8; 32]) -> String {
        bs58::encode(identifier).into_string()
    }

    fn client_path(&self, identifier: [u8; 32], network: Network) -> PathBuf {
        let ident = Self::identifier_to_string(identifier);
        let network_prefix = match network {
            Network::Testnet => "testnet",
            Network::Localnet => "localnet",
        };
        let dir_name = format!("{}_{}", network_prefix, ident);
        self.store_path.join(dir_name)
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
        identifier: [u8; 32],
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let mut all_accounts = Vec::new();

        // Check both testnet and localnet directories
        for network in [Network::Testnet, Network::Localnet] {
            let path = self.client_path(identifier, network);
            let account_id_file = path.join("account_id.txt");

            if !account_id_file.exists() {
                continue;
            }

            let contents = std::fs::read_to_string(&account_id_file)?;

            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Parse the bech32 to extract the network
                match miden_objects::address::Address::from_bech32(line) {
                    Ok((network_id, _)) => {
                        let network_str = network_id.as_str().to_string();
                        all_accounts.push((line.to_string(), network_str));
                    }
                    Err(_e) => {
                        // Skip invalid entries
                        continue;
                    }
                }
            }
        }

        Ok(all_accounts)
    }

    pub async fn get_client(
        &mut self,
        identifier: [u8; 32],
        network: Network,
    ) -> Result<ClientHandle, Box<dyn std::error::Error>> {
        // Check if we already have a client for this identifier and network in cache
        if let Some(client_handle) = self.clients.get(&(identifier, network)) {
            return Ok(client_handle.clone());
        }

        let path = self.client_path(identifier, network);

        // Client not in memory, create a new one on its dedicated runtime
        let client_handle = ClientHandle::spawn(path, network).await?;

        // Cache the client handle
        self.clients
            .insert((identifier, network), client_handle.clone());

        Ok(client_handle)
    }

    pub async fn new_account(
        &mut self,
        identifier: [u8; 32],
        _account_type: AccountType,
        network: Network,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let path = self.client_path(identifier, network);
        Self::check_or_create(&path)?;

        // Get or create client handle
        let client_handle = self.get_client(identifier, network).await?;

        // Create account via the client handle
        let account = client_handle
            .create_account()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create account: {}", e))?;

        // Convert account ID to bech32
        let account_id = account.id();
        let address = miden_objects::address::AccountIdAddress::new(
            account_id,
            miden_objects::address::AddressInterface::BasicWallet,
        );
        let network_id = network.to_network_id();
        let account_id_bech32 =
            miden_objects::address::Address::from(address).to_bech32(network_id);

        // Persist to file
        let account_id_file = path.join("account_id.txt");
        use std::fs::OpenOptions;
        use std::io::Write;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&account_id_file)?;
        writeln!(file, "{}", account_id_bech32)?;

        Ok(account_id_bech32)
    }

    pub async fn create_private_note(
        &mut self,
        identifier: [u8; 32],
        network: Network,
        account_id_bech32: String,
        order: mosaic_fi::note::Order,
    ) -> Result<MosaicNote, Box<dyn std::error::Error>> {
        // Get the client handle
        let client_handle = self.get_client(identifier, network).await?;

        // Parse the account ID from bech32
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

        // Verify the account exists in the client
        let _account_record = client_handle
            .get_account(account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get account: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("Account not found: {}", account_id_bech32))?;

        // Compile the note using the account ID
        let mosaic_note = mosaic_fi::note::compile_note_from_account_id(account_id, order)?;

        // Automatically commit the note to the network
        client_handle
            .commit_note(account_id, mosaic_note.miden_note.miden_note_hex.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to commit note: {}", e))?;

        Ok(mosaic_note)
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
