use miden_client::{account::AccountId, keystore::FilesystemKeyStore};
use rand::rngs::StdRng;
use std::sync::{Arc, Mutex};

use crate::Network;

pub struct Account<T> {
    pub miden_account: miden_client::account::Account,
    pub miden_client: Arc<Mutex<miden_client::Client<T>>>,
    pub network: Network,
}

impl<T> Account<T> {
    pub fn miden_account_id(&self) -> AccountId {
        self.miden_account.id()
    }

    pub fn miden_account_id_bech32(&self) -> String {
        let account_id = self.miden_account.id();
        let network_id = self.network.to_network_id();
        account_id.to_bech32(network_id)
    }
}

impl Account<FilesystemKeyStore<StdRng>> {}
