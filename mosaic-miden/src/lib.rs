pub mod account;
pub mod client;
pub mod note;
pub mod transaction;
pub mod version;

use miden_objects::account::NetworkId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Network {
    Testnet,
    Localnet,
}

impl Network {
    pub fn to_network_id(&self) -> NetworkId {
        match self {
            Network::Testnet => NetworkId::Testnet,
            // Network ID used for local instances of the node
            Network::Localnet => NetworkId::new("mlcl").expect("mlcl should be a valid network ID"),
        }
    }
}
