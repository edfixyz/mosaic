pub mod account;
pub mod client;
pub mod note;
pub mod store;
pub mod symbol;
pub mod transaction;
pub mod version;

pub type MidenTransactionId = String;

use miden_objects::account::NetworkId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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

    pub fn from_network_id(network_id: NetworkId) -> Option<Self> {
        if network_id == NetworkId::Testnet {
            return Some(Network::Testnet);
        }

        if network_id == Network::Localnet.to_network_id() {
            return Some(Network::Localnet);
        }

        None
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Network::Testnet => "Testnet",
            Network::Localnet => "Localnet",
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "testnet" => Ok(Network::Testnet),
            "localnet" => Ok(Network::Localnet),
            _ => Err(format!(
                "Unsupported network '{s}'. Expected 'Testnet' or 'Localnet'."
            )),
        }
    }
}

impl TryFrom<&str> for Network {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl TryFrom<String> for Network {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl From<Network> for String {
    fn from(value: Network) -> Self {
        value.as_str().to_string()
    }
}

impl<'a> TryFrom<&'a String> for Network {
    type Error = String;

    fn try_from(value: &'a String) -> Result<Self, Self::Error> {
        value.parse()
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
    /// Storage mode: "Private" or "Public"
    pub storage_mode: String,
    /// Account type: "Client", "Desk", "Liquidity", or "Faucet"
    pub account_type: String,
    /// List of assets held by the account
    pub assets: Vec<AssetData>,
}
