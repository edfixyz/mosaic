use crate::Network;
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
    transaction::TransactionRequestBuilder,
};
use miden_lib::{
    account::{auth::AuthRpoFalcon512, faucets::BasicFungibleFaucet},
    note::create_p2id_note,
};
use miden_objects::{
    Felt,
    account::{AccountBuilder, AccountStorageMode, AccountType as MidenAccountType},
    asset::{FungibleAsset, TokenSymbol},
    note::NoteType as MidenNoteType,
    transaction::OutputNote,
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
    let sqlite_path = path.join("miden_store.sqlite3");
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
    FundAccount {
        faucet_account_id: AccountId,
        target_account_id: AccountId,
        amount: u64,
        respond_to: oneshot::Sender<Result<(), String>>,
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
        respond_to: oneshot::Sender<Result<(), String>>,
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
                ClientCommand::FundAccount {
                    faucet_account_id,
                    target_account_id,
                    amount,
                    respond_to,
                } => {
                    let result = Self::fund_account_impl(
                        &mut client,
                        faucet_account_id,
                        target_account_id,
                        amount,
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

    /// Implementation of fund account logic using P2ID notes
    async fn fund_account_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        faucet_account_id: AccountId,
        target_account_id: AccountId,
        amount: u64,
    ) -> Result<(), String> {
        // Create a fungible asset from the faucet
        let fungible_asset = FungibleAsset::new(faucet_account_id, amount)
            .map_err(|e| format!("Failed to create fungible asset: {}", e))?;

        // Create a P2ID note to send the asset to the target account
        let p2id_note = create_p2id_note(
            faucet_account_id,
            target_account_id,
            vec![fungible_asset.into()],
            MidenNoteType::Private,
            Felt::new(0),
            client.rng(),
        )
        .map_err(|e| format!("Failed to create P2ID note: {}", e))?;

        // Create a transaction request with the P2ID note as output
        let output_notes = vec![OutputNote::Full(p2id_note)];
        let transaction_request = TransactionRequestBuilder::new()
            .own_output_notes(output_notes)
            .build()
            .map_err(|e| format!("Failed to build transaction request: {}", e))?;

        // Execute the transaction
        let tx_execution_result = client
            .new_transaction(faucet_account_id, transaction_request)
            .await
            .map_err(|e| format!("Failed to execute transaction: {}", e))?;

        // Submit the transaction
        client
            .submit_transaction(tx_execution_result)
            .await
            .map_err(|e| format!("Failed to submit transaction: {}", e))?;

        Ok(())
    }

    /// Implementation of note commitment logic
    async fn commit_note_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        account_id: AccountId,
        note_hex: &str,
    ) -> Result<(), String> {
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
    pub async fn commit_note(&self, account_id: AccountId, note_hex: String) -> Result<(), String> {
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

    /// Fund an account using P2ID note
    pub async fn fund_account(
        &self,
        faucet_account_id: AccountId,
        target_account_id: AccountId,
        amount: u64,
    ) -> Result<(), String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::FundAccount {
                faucet_account_id,
                target_account_id,
                amount,
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
