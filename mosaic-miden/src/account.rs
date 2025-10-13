use miden_client::{account::AccountId, keystore::FilesystemKeyStore};
use miden_objects::address::{AccountIdAddress, Address, AddressInterface};
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
        // Convert AccountId to bech32 format using the Address type
        let account_id = self.miden_account.id();
        let address = AccountIdAddress::new(account_id, AddressInterface::BasicWallet);
        let network_id = self.network.to_network_id();
        Address::from(address).to_bech32(network_id)
    }
}

impl Account<FilesystemKeyStore<StdRng>> {}
