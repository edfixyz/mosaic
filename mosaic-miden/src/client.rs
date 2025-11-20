use crate::{MidenTransactionId, Network, symbol::encode_symbol};
use miden_client::{
    Client,
    account::{AccountHeader, AccountId, component::BasicWallet},
    auth::AuthSecretKey,
    builder::ClientBuilder,
    keystore::FilesystemKeyStore,
    rpc::{Endpoint, GrpcClient},
    store::{AccountRecord, AccountStatus},
    sync::SyncSummary,
    transaction::TransactionKernel,
};
use miden_client_sqlite_store::SqliteStore;
use miden_lib::account::{
    auth::{AuthRpoFalcon512, NoAuth},
    faucets::BasicFungibleFaucet,
};
use miden_objects::{
    Felt, Word,
    account::{
        AccountBuilder, AccountComponent, AccountStorageMode, AccountType as MidenAccountType,
        StorageMap, StorageSlot,
    },
    assembly::Assembler,
    asset::TokenSymbol,
};
use rand::{Rng, RngCore, rngs::StdRng};
use std::sync::Arc;
use std::{
    env,
    path::{Path, PathBuf},
};
use tokio::sync::{mpsc, oneshot};

type DeskAccountArtifacts = (
    miden_client::account::Account,
    AuthSecretKey,
    Option<String>,
);

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
    let rpc = Arc::new(GrpcClient::new(&endpoint, timeout_ms));
    let keystore_path = path.join("keystore");
    let sqlite_path = path.join("miden.sqlite3");
    let keystore = Arc::new(FilesystemKeyStore::new(keystore_path)?);

    let store = Arc::new(SqliteStore::new(sqlite_path).await?);
    let client: Client<FilesystemKeyStore<StdRng>> = ClientBuilder::new()
        .rpc(rpc)
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .store(store)
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
        respond_to:
            oneshot::Sender<Result<(miden_client::account::Account, AuthSecretKey), String>>,
    },
    CreateFaucetAccount {
        token_symbol: String,
        decimals: u8,
        max_supply: u64,
        respond_to:
            oneshot::Sender<Result<(miden_client::account::Account, AuthSecretKey), String>>,
    },
    CreateDeskAccount {
        quote_symbol: String,
        quote_account: String,
        base_symbol: String,
        base_account: String,
        owner_account: AccountId,
        respond_to: oneshot::Sender<Result<DeskAccountArtifacts, String>>,
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
        network: Network,
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
                ClientCommand::CreateDeskAccount {
                    quote_symbol,
                    quote_account,
                    base_symbol,
                    base_account,
                    owner_account,
                    respond_to,
                } => {
                    let result = Self::create_desk_account_impl(
                        &mut client,
                        &base_symbol,
                        &base_account,
                        &quote_symbol,
                        &quote_account,
                        owner_account,
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
                    network,
                    respond_to,
                } => {
                    let result = Self::get_account_status_impl(&client, account_id, network).await;
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
    ) -> Result<(miden_client::account::Account, AuthSecretKey), String> {
        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let key_pair = AuthSecretKey::new_rpo_falcon512_with_rng(client.rng());
        let auth_component = AuthRpoFalcon512::new(key_pair.public_key().to_commitment());

        let builder = AccountBuilder::new(init_seed)
            .account_type(MidenAccountType::RegularAccountUpdatableCode)
            .storage_mode(AccountStorageMode::Private)
            .with_auth_component(auth_component)
            .with_component(BasicWallet);

        let miden_account = builder
            .build()
            .map_err(|e| format!("Account build failed: {}", e))?;

        client
            .add_account(&miden_account, false)
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
    ) -> Result<(miden_client::account::Account, AuthSecretKey), String> {
        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);

        let symbol =
            TokenSymbol::new(token_symbol).map_err(|e| format!("Invalid token symbol: {}", e))?;
        let max_supply_felt = Felt::new(max_supply);

        let key_pair = AuthSecretKey::new_rpo_falcon512_with_rng(client.rng());
        let auth_component = AuthRpoFalcon512::new(key_pair.public_key().to_commitment());

        let builder = AccountBuilder::new(init_seed)
            .account_type(MidenAccountType::FungibleFaucet)
            .storage_mode(AccountStorageMode::Public)
            .with_auth_component(auth_component)
            .with_component(
                BasicFungibleFaucet::new(symbol, decimals, max_supply_felt)
                    .map_err(|e| format!("Failed to create faucet component: {}", e))?,
            );

        let miden_account = builder
            .build()
            .map_err(|e| format!("Account build failed: {}", e))?;

        client
            .add_account(&miden_account, false)
            .await
            .map_err(|e| format!("Add account failed: {}", e))?;
        client.sync_state().await?;

        Ok((miden_account, key_pair))
    }

    /// Implementation of faucet account creation logic
    async fn create_desk_account_impl(
        client: &mut Client<FilesystemKeyStore<StdRng>>,
        base_symbol: &str,
        base_account: &str,
        quote_symbol: &str,
        quote_account: &str,
        owner_account: AccountId,
    ) -> Result<DeskAccountArtifacts, String> {
        let _ = owner_account;
        if base_account == quote_account {
            return Err("Base and quote accounts must be different".to_string());
        }

        let (base_network_id, base_account_id) = AccountId::from_bech32(base_account)
            .map_err(|e| format!("Invalid base account '{}': {}", base_account, e))?;

        let (quote_network_id, quote_account_id) = AccountId::from_bech32(quote_account)
            .map_err(|e| format!("Invalid quote account '{}': {}", quote_account, e))?;

        if base_network_id != quote_network_id {
            return Err(format!(
                "Base and quote accounts must be on the same network ({} vs {})",
                base_network_id, quote_network_id
            ));
        }

        let network = Network::from_network_id(base_network_id)
            .ok_or_else(|| "Unsupported network id".to_string())?;

        tracing::info!(
            base_symbol,
            base_account,
            quote_symbol,
            quote_account,
            ?network,
            "Creating desk account"
        );

        let key_pair = AuthSecretKey::new_rpo_falcon512_with_rng(client.rng());
        let mut init_seed = [0u8; 32];
        client.rng().fill_bytes(&mut init_seed);
        let assembler: Assembler = TransactionKernel::assembler().with_debug_mode(true);

        let book_code = include_str!("../../mosaic-fi/masm/accounts/book.masm").to_string();

        let zero_word = || Word::from([Felt::new(0); 4]);
        let base_symbol_upper = base_symbol.to_ascii_uppercase();
        let quote_symbol_upper = quote_symbol.to_ascii_uppercase();

        let base_symbol_word = Word::from(
            encode_symbol(&base_symbol_upper, &base_account_id)
                .map_err(|e| format!("Invalid base symbol: {}", e))?,
        );
        let quote_symbol_word = Word::from(
            encode_symbol(&quote_symbol_upper, &quote_account_id)
                .map_err(|e| format!("Invalid quote symbol: {}", e))?,
        );

        let book_component = AccountComponent::compile(
            book_code,
            assembler,
            vec![
                StorageSlot::Value(zero_word()),
                StorageSlot::Value(base_symbol_word),
                StorageSlot::Value(quote_symbol_word),
                StorageSlot::Value(zero_word()),
                StorageSlot::Value(zero_word()), // Status
                // Sell
                StorageSlot::Value(zero_word()),
                StorageSlot::Map(StorageMap::new()),
                StorageSlot::Value(zero_word()),
                StorageSlot::Value(zero_word()),
                StorageSlot::Value(zero_word()),
                // Buy
                StorageSlot::Value(zero_word()),
                StorageSlot::Map(StorageMap::new()),
                StorageSlot::Value(zero_word()),
                StorageSlot::Value(zero_word()),
                StorageSlot::Value(zero_word()),
            ],
        )
        .map_err(|e| format!("Failed to compile desk component: {}", e))?
        .with_supports_all_types();

        let book_contract = AccountBuilder::new(init_seed)
            .account_type(MidenAccountType::RegularAccountImmutableCode)
            .storage_mode(AccountStorageMode::Public)
            .with_component(BasicWallet)
            .with_component(book_component)
            .with_auth_component(NoAuth)
            .build()
            .map_err(|e| format!("Failed to build desk account: {}", e))?;

        client
            .add_account(&book_contract, false)
            .await
            .map_err(|e| format!("Failed to add desk account: {}", e))?;
        client.sync_state().await?;

        // Workaround Start ===============================================================================================
        let abstract_note = crate::note::MidenAbstractNote {
            version: "MOSAIC 2025.10 MIDEN 0.11".to_string(),
            note_type: crate::note::NoteType::Private,
            program: include_str!("../../mosaic-fi/masm/notes/desk_update_status.masm").to_string(),
            libraries: vec![(
                "external_contract::book".to_string(),
                include_str!("../../mosaic-fi/masm/accounts/book.masm").to_string(),
            )],
        };
        let intent: [u64; 4] = [
            client.rng().random(),
            client.rng().random(),
            client.rng().random(),
            client.rng().random(),
        ];
        let inputs = vec![
            ("intent".to_string(), crate::note::Value::Word(intent)),
            ("status".to_string(), crate::note::Value::Word([1, 1, 1, 1])),
        ];
        let note = crate::note::compile_note(abstract_note, owner_account, zero_word(), inputs)
            .map_err(|e| format!("Failed to compile note: {}", e))?;
        let _ = crate::note::commit_note(client, owner_account, &note)
            .await
            .map_err(|e| format!("Failed to commit the note: {}", e))?;
        client.sync_state().await?;
        Self::consume_note_impl(client, book_contract.id(), &note.miden_note_hex)
            .await
            .map_err(|e| format!("Failed to consume note: {}", e))?;
        // Workaround End ===============================================================================================

        let account_id = book_contract.id();
        let network_id = network.to_network_id();
        let bech32 = account_id.to_bech32(network_id);

        let market_url = env::var("MOSAIC_SERVER")
            .ok()
            .map(|base| format!("{}/desk/{}", base.trim_end_matches('/'), bech32));

        Ok((book_contract, key_pair, market_url))
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
            .unauthenticated_input_notes(vec![(note, None)])
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
        let tx_id = client
            .submit_new_transaction(account_id, tx_request)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    account_id = %account_id,
                    "Failed to execute transaction"
                );
                format!("Failed to execute transaction: {:?}", e)
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
        network: Network,
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
        let storage_mode = if account_id.is_private() {
            "Private"
        } else {
            "Public"
        };

        // Iterate through assets
        let mut assets = Vec::new();
        for asset in account.vault().assets() {
            match asset {
                Asset::Fungible(fungible_asset) => {
                    let faucet_id = fungible_asset.faucet_id();
                    let faucet_bech32 = faucet_id.to_bech32(network.to_network_id());

                    assets.push(crate::AssetData {
                        faucet: faucet_bech32,
                        amount: fungible_asset.amount(),
                        fungible: true,
                    });
                }
                Asset::NonFungible(_non_fungible_asset) => {
                    // For non-fungible assets, we'll use a placeholder
                    // In a production system, you'd want to properly handle this
                    assets.push(crate::AssetData {
                        faucet: "non-fungible".to_string(),
                        amount: 1,
                        fungible: false,
                    });
                }
            }
        }

        let account_id_bech32 = account_id.to_bech32(network.to_network_id());

        Ok(crate::AccountStatusData {
            account_id: account_id_bech32,
            storage_mode: storage_mode.to_string(),
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
            .add_key(&key_pair)
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
            .add_key(&key_pair)
            .map_err(|e| format!("Failed to store key: {}", e))?;

        Ok(account)
    }

    /// Create a new desk account in the client
    /// Returns the account (the secret key is automatically stored in the keystore)
    pub async fn create_desk_account(
        &self,
        base_symbol: String,
        base_account: String,
        quote_symbol: String,
        quote_account: String,
        owner_account: AccountId,
    ) -> Result<(miden_client::account::Account, Option<String>), String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::CreateDeskAccount {
                quote_symbol,
                quote_account,
                base_symbol,
                base_account,
                owner_account,
                respond_to,
            })
            .map_err(|_| "Client thread has shut down".to_string())?;

        let (account, key_pair, market_url) = response_rx
            .await
            .map_err(|_| "Client thread dropped response".to_string())??;

        self.keystore
            .add_key(&key_pair)
            .map_err(|e| format!("Failed to store key: {}", e))?;

        Ok((account, market_url))
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
        network: Network,
    ) -> Result<crate::AccountStatusData, String> {
        let (respond_to, response_rx) = oneshot::channel();

        self.command_tx
            .send(ClientCommand::GetAccountStatus {
                account_id,
                network,
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
