pub mod account;
pub mod note;

use serde::{Deserialize, Serialize};

pub use account::{AccountOrder, AccountOrderResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Client,
    Desk,
    Liquidity,
    Faucet,
}

/// Currency definition with code and issuer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Currency {
    /// Currency code (e.g., "BTC", "USDC")
    pub code: String,
    /// Issuer account ID in bech32 format
    pub issuer: String,
}

/// Market definition with base and quote currencies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Market {
    /// Base currency (e.g., BTC in BTC/USDC)
    pub base: Currency,
    /// Quote currency (e.g., USDC in BTC/USDC)
    pub quote: Currency,
}
