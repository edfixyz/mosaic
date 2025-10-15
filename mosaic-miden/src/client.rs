use crate::{MidenTransactionId, Network};
use miden_client::{
    Client,
    account::{AccountHeader, AccountId, component::BasicWallet},
    auth::AuthSecretKey,
    builder::ClientBuilder,
    crypto::SecretKey,
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, TonicRpcClient},
    store::{AccountRecord, AccountStatus},
    sync::SyncSummary,
};
use miden_lib::account::{auth::AuthRpoFalcon512, faucets::BasicFungibleFaucet};
use miden_objects::{
    Felt,
    account::{AccountBuilder, AccountStorageMode, AccountType as MidenAccountType},
    asset::TokenSymbol,
};
use rand::{RngCore, rngs::StdRng};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};

pub async fn create_client(
    path: &Path,
    network: Network,
) -> Result<
    (
        Client<FilesystemKeyStore<StdRng>>,
        Arc<FilesystemKeyStore<StdRng>>,
    ),
    Box<dyn std::error::Error>,
> {
    let endpoint = match network {
        Network::Testnet => Endpoint::testnet(),
        Network::Localnet => Endpoint::localhost(),
    };

    let timeout_ms = 10_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));
    let keystore_path = path.join("keystore");
    let sqlite_path = path.join("miden.sqlite3");
    let keystore = Arc::new(FilesystemKeyStore::new(keystore_path)?);

    let client = ClientBuilder::new()
        .rpc(rpc_api)
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .sqlite_store(sqlite_path.to_str().ok_or("path contains invalid UTF-8")?)
        .build()
        .await?;

    Ok((client, keystore))
}

/// Commands that can be sent to the client thread
pub enum ClientCommand {
    Sync {
        respond_to: oneshot::Sender<Result<SyncSummary, String>>,
    },
    CreateAccount {
        respond_to: oneshot::Sender<Result<(miden_client::account::Account, SecretKey), String>>,
    },
    CreateFaucetAccount {
        token_symbol: String,
        decimals: u8,
        max_supply: u64,
        respond_to: oneshot::Sender<Result<(miden_client::account::Account, SecretKey), String>>,
    },
    GetAccount {
        account_id: AccountId,
        respond_to: oneshot::Sender<Result<Option<AccountRecord>, String>>,
    },
    ListAccounts {
        respond_to: oneshot::Sender<Result<Vec<(AccountHeader, AccountStatus)>, String>>,
    },
    CommitNote {
        account_id: AccountId,
        note_hex: String,
        respond_to: oneshot::Sender<Result<MidenTransactionId, String>>,
    },
    ConsumeNote {
        account_id: AccountId,
        note_hex: String,
        respond_to: oneshot::Sender<Result<MidenTransactionId, String>>,
    },
    GetAccountStatus {
        account_id: AccountId,
        respond_to: oneshot::Sender<Result<crate::AccountStatusData, String>>,
    },
    Shutdown,
}

/// Handle to communicate with a client running in a dedicated thread
#[derive(Clone)]
pub struct ClientHandle {
    command_tx: mpsc::UnboundedSender<ClientCommand>,
    keystore: Arc<FilesystemKeyStore<StdRng>>,
}

impl ClientHandle {
    /// Create a new client handle and spawn a dedicated thread for the client
    pub async fn spawn(
        path: PathBuf,
        network: Network,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = oneshot::channel();

        // Spawn a dedicated thread for this client
        std::thread::spawn(move || {
            // Create a single-threaded runtime for this thread
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime");

            // Initialize the client inside the dedicated thread
            rt.block_on(async move {
                match create_client(path.as_path(), network).await {
                    Ok((client, keystore)) => {
                        let _ = ready_tx.send(Ok(keystore.clone()));
                        Self::run_client_loop(client, command_rx).await;
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                    }
                }
            });
        });

        let keystore = match ready_rx.await {
            Ok(Ok(keystore)) => keystore,
            Ok(Err(err_msg)) => {
                return Err(Box::new(std::io::Error::other(err_msg)));
            }
            Err(_) => {
                return Err(Box::new(std::io::Error::other(
                    "Client thread failed to initialize",
                )));
            }
        };

        Ok(ClientHandle {
            command_tx,
            keystore,
        })
    }

    /// Main event loop for the client thread
    async fn run_client_loop(
        mut client: Client<FilesystemKeyStore<StdRng>>,
        mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    ) {
        while let Some(command) = command_rx.recv().await {
            match command {
                ClientCommand::Sync { respond_to } => {
                    let result = client
                        .sync_state()
                        .await
                        .map_err(|e| {
                            format!("Sync failed: {}. If using Localnet, ensure a Miden node is running on localhost:57291", e)
                        });

                    let _ = respond_to.send(result);
                }
                ClientCommand::CreateAccount { respond_to } => {
                    let result = Self::create_account_impl(&mut client).await;
                    let _ = respond_to.send(result);
                }
                ClientCommand::CreateFaucetAccount {
                    token_symbol,
                    decimals,
                    max_supply,
                    respond_to,
                } => {
                    let result = Self::create_faucet_account_impl(
                        &mut client,
                        &token_symbol,
                        decimals,
                        max_supply,
                    )
                    .await;
                    let _ = respond_to.send(result);
                }
                ClientCommand::GetAccount {
                    account_id,
                    respond_to,
                } => {
                    let result = client
                        .get_account(account_id)
                        .await
                        .map_err(|e| format!("Get account failed: {}", e));
                    let _ = respond_to.send(result);
                }
                ClientCommand::ListAccounts { respond_to } => {
                    let result = client
                        .get_account_headers()
                        .await
                        .map_err(|e| format!("List accounts failed: {}", e));
                    let _ = respond_to.send(result);
                }
                ClientCommand::CommitNote {
                    account_id,
                    note_hex,
                    respond_to,
                } => {
                    let result = Self::commit_note_impl(&mut client, account_id, &note_hex).await;
                    let _ = respond_to.send(result);
                }
                ClientCommand::ConsumeNote {
                    account_id,
                    note_hex,
                    respond_to,
                } => {
                    let result = Self::consume_note_impl(&mut client, account_id, &note_hex).await;
                    let _ = respond_to.send(result);
                }
                ClientCommand::GetAccountStatus {
                    account_id,
                    respond_to,
                } => {
                    let result = Self::get_account_status_impl(&client, account_id).await;
                    let _ = respond_to.send(result);
                }
                ClientCommand::Shutdown => {
                    break;
                }
            }
        }
    }

    /// Implementation of account creation logic
    async fn create_account_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
    ) -> Result<(miden_client::account::Account, SecretKey), String> {
        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let key_pair = SecretKey::with_rng(client.rng());

        let builder = AccountBuilder::new(init_seed)
            .account_type(MidenAccountType::RegularAccountUpdatableCode)
            .storage_mode(AccountStorageMode::Private)
            .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key()))
            .with_component(BasicWallet);

        let (miden_account, seed) = builder
            .build()
            .map_err(|e| format!("Account build failed: {}", e))?;

        client
            .add_account(&miden_account, Some(seed), false)
            .await
            .map_err(|e| format!("Add account failed: {}", e))?;
        client.sync_state().await?;

        Ok((miden_account, key_pair))
    }

    /// Implementation of faucet account creation logic
    async fn create_faucet_account_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        token_symbol: &str,
        decimals: u8,
        max_supply: u64,
    ) -> Result<(miden_client::account::Account, SecretKey), String> {
        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let symbol =
            TokenSymbol::new(token_symbol).map_err(|e| format!("Invalid token symbol: {}", e))?;
        let max_supply_felt = Felt::new(max_supply);

        let key_pair = SecretKey::with_rng(client.rng());

        let builder = AccountBuilder::new(init_seed)
            .account_type(MidenAccountType::FungibleFaucet)
            .storage_mode(AccountStorageMode::Public)
            .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key()))
            .with_component(
                BasicFungibleFaucet::new(symbol, decimals, max_supply_felt)
                    .map_err(|e| format!("Failed to create faucet component: {}", e))?,
            );

        let (miden_account, seed) = builder
            .build()
            .map_err(|e| format!("Account build failed: {}", e))?;

        client
            .add_account(&miden_account, Some(seed), false)
            .await
            .map_err(|e| format!("Add account failed: {}", e))?;
        client.sync_state().await?;

        Ok((miden_account, key_pair))
    }

    /// Implementation of note commitment logic
    async fn commit_note_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        account_id: AccountId,
        note_hex: &str,
    ) -> Result<MidenTransactionId, String> {
        use crate::note::MidenNote;

        let miden_note = MidenNote {
            version: crate::version::VERSION_STRING.to_string(),
            note_type: crate::note::NoteType::Private,
            miden_note_hex: note_hex.to_string(),
        };

        crate::note::commit_note(client, account_id, &miden_note)
            .await
            .map_err(|e| format!("Commit note failed: {}", e))
    }

    /// Implementation of note consumption logic
    async fn consume_note_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        account_id: AccountId,
        note_hex: &str,
    ) -> Result<MidenTransactionId, String> {
        use miden_client::transaction::TransactionRequestBuilder;
        use miden_lib::utils::Deserializable;

        tracing::info!(
            account_id = %account_id,
            note_hex_length = note_hex.len(),
            "Starting note consumption"
        );

        // Decode note hex
        let note_bytes = hex::decode(note_hex).map_err(|e| {
            tracing::error!(
                error = %e,
                account_id = %account_id,
                note_hex = %note_hex,
                note_hex_length = note_hex.len(),
                "Failed to decode note hex"
            );
            format!("Failed to decode note hex: {}", e)
        })?;

        tracing::info!(
            account_id = %account_id,
            note_bytes_length = note_bytes.len(),
            "Successfully decoded note hex"
        );

        // Deserialize note
        let note = miden_client::note::Note::read_from_bytes(&note_bytes).map_err(|e| {
            tracing::error!(
                error = %e,
                account_id = %account_id,
                note_bytes_length = note_bytes.len(),
                note_bytes_hex = %hex::encode(&note_bytes),
                "Failed to deserialize note from bytes"
            );
            format!("Failed to deserialize note: {}", e)
        })?;

        let note_id = note.id();
        tracing::info!(
            account_id = %account_id,
            note_id = %note_id,
            note_metadata = ?note.metadata(),
            note_assets = ?note.assets(),
            "Successfully deserialized note"
        );

        // Build transaction request to consume the note
        let tx_request = TransactionRequestBuilder::new()
            //.build_consume_notes(vec![note_id])
            .unauthenticated_input_notes(vec!((note, None)))
            .build()
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    "Failed to build transaction"
                );
                format!("Failed to build transaction: {:?}", e)
            })?;

        tracing::info!(
            account_id = %account_id,
            note_id = %note_id,
            "Successfully built transaction request"
        );

        // Execute transaction
        let tx_result = client
            .new_transaction(account_id, tx_request)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    account_id = %account_id,
                    "Failed to execute transaction"
                );
                format!("Failed to execute transaction: {:?}", e)
            })?;

        let tx_id = tx_result.executed_transaction().id();
        tracing::info!(
            transaction_id = %tx_id,
            account_id = %account_id,
            note_id = %note_id,
            "Successfully executed transaction"
        );

        // Submit transaction
        client.submit_transaction(tx_result).await.map_err(|e| {
            tracing::error!(
                error = %e,
                transaction_id = %tx_id,
                account_id = %account_id,
                note_id = %note_id,
                "Failed to submit transaction"
            );
            format!("Failed to submit transaction: {}", e)
        })?;

        tracing::info!(
            transaction_id = %tx_id,
            account_id = %account_id,
            note_id = %note_id,
            "Successfully submitted transaction"
        );

        Ok(format!("{}", tx_id))
    }

    /// Implementation of getting account status
    async fn get_account_status_impl(
        client: &Client<FilesystemKeyStore<StdRng>>,
        account_id: AccountId,
    ) -> Result<crate::AccountStatusData, String> {
        use miden_objects::asset::Asset;

        // Get account
        let account_record = client
            .get_account(account_id)
            .await
            .map_err(|e| format!("Failed to get account: {}", e))?
            .ok_or_else(|| format!("Account not found: {}", account_id))?;

        let account: miden_client::account::Account = account_record.into();

        // Get account type from the account ID's storage mode
        let account_type = if account_id.is_private() {
            "Private"
        } else {
            "Public"
        };

        // For bech32 encoding, we need a network ID. Since we don't have it in this context,
        // we'll use hex representation for account IDs and faucet IDs

        // Iterate through assets
        let mut assets = Vec::new();
        for asset in account.vault().assets() {
            match asset {
                Asset::Fungible(fungible_asset) => {
                    let faucet_id = fungible_asset.faucet_id();
                    // Use hex format instead of bech32 since we don't have network context
                    let faucet_hex = format!("0x{}", faucet_id.to_hex());

                    assets.push(crate::AssetData {
                        faucet: faucet_hex,
                        amount: fungible_asset.amount(),
                        fungible: true,
                    });
                }
                Asset::NonFungible(_non_fungible_asset) => {
                    // For non-fungible assets, we'll skip them for now as they're more complex
                    // and the faucet_id_prefix doesn't directly convert to AccountId
                    // In a production system, you'd want to properly handle this
                    assets.push(crate::AssetData {
                        faucet: "non-fungible".to_string(),
                        amount: 1,
                        fungible: false,
                    });
                }
            }
        }

        // Get account ID in hex format
        let account_id_hex = format!("0x{}", account_id.to_hex());

        Ok(crate::AccountStatusData {
            account_id: account_id_hex,
            storage_mode: account_type.to_string(),
            account_type: String::new(), // Will be filled in by the serve layer from the store
            assets,
        })
    }

    /// Request the client to sync its state
    pub async fn sync(&self) -> Result<SyncSummary, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::Sync { respond_to })
            .map_err(|_| "Client thread has shut down".to_string())?;

        response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())?
    }

    /// Create a new account in the client
    /// Returns the account (the secret key is automatically stored in the keystore)
    pub async fn create_account(&self) -> Result<miden_client::account::Account, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::CreateAccount { respond_to })
            .map_err(|_| "Client thread has shut down".to_string())?;

        let (account, key_pair) = response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())??;

        // Store the key in the keystore
        self.keystore
            .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
            .map_err(|e| format!("Failed to store key: {}", e))?;

        Ok(account)
    }

    /// Create a new faucet account in the client
    /// Returns the account (the secret key is automatically stored in the keystore)
    pub async fn create_faucet_account(
        &self,
        token_symbol: String,
        decimals: u8,
        max_supply: u64,
    ) -> Result<miden_client::account::Account, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::CreateFaucetAccount {
                token_symbol,
                decimals,
                max_supply,
                respond_to,
            })
            .map_err(|_| "Client thread has shut down".to_string())?;

        let (account, key_pair) = response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())??;

        // Store the key in the keystore
        self.keystore
            .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
            .map_err(|e| format!("Failed to store key: {}", e))?;

        Ok(account)
    }

    /// Get an account by ID from the client store
    pub async fn get_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<AccountRecord>, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::GetAccount {
                account_id,
                respond_to,
            })
            .map_err(|_| "Client thread has shut down".to_string())?;

        response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())?
    }

    /// List all accounts in the client store
    pub async fn list_accounts(&self) -> Result<Vec<(AccountHeader, AccountStatus)>, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::ListAccounts { respond_to })
            .map_err(|_| "Client thread has shut down".to_string())?;

        response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())?
    }

    /// Commit a note to the network
    pub async fn commit_note(
        &self,
        account_id: AccountId,
        note_hex: String,
    ) -> Result<MidenTransactionId, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::CommitNote {
                account_id,
                note_hex,
                respond_to,
            })
            .map_err(|_| "Client thread has shut down".to_string())?;

        response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())?
    }

    /// Consume a note and execute the transaction
    /// Returns the transaction ID
    pub async fn consume_note(
        &self,
        account_id: AccountId,
        note_hex: String,
    ) -> Result<MidenTransactionId, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::ConsumeNote {
                account_id,
                note_hex,
                respond_to,
            })
            .map_err(|_| "Client thread has shut down".to_string())?;

        response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())?
    }

    /// Get account status including assets
    pub async fn get_account_status(
        &self,
        account_id: AccountId,
    ) -> Result<crate::AccountStatusData, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::GetAccountStatus {
                account_id,
                respond_to,
            })
            .map_err(|_| "Client thread has shut down".to_string())?;

        response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())?
    }

    /// Shutdown the client thread gracefully
    pub fn shutdown(&self) {
        let _ = self.command_tx.send(ClientCommand::Shutdown);
    }
}
