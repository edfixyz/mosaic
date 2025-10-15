pub mod account;
pub mod client;
pub mod note;
pub mod store;
pub mod transaction;
pub mod version;

use miden_objects::account::NetworkId;
use serde::Serialize;

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

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct AssetData {
    /// Faucet account ID in bech32 format
    pub faucet: String,
    /// Amount of the asset
    pub amount: u64,
    /// Whether this is a fungible asset
    pub fungible: bool,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct AccountStatusData {
    /// Account ID in bech32 format
    pub account_id: String,
    /// Account type: "Private" or "Public"
    #[serde(rename = "type")]
    pub account_type: String,
    /// List of assets held by the account
    pub assets: Vec<AssetData>,
}
