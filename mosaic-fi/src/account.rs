use crate::Market;
use mosaic_miden::Network;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AccountOrder {
    CreateClient {
        network: Network,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    CreateDesk {
        network: Network,
        market: Market,
        owner_account: String,
    },
    CreateFaucet {
        network: Network,
        token_symbol: String,
        decimals: u8,
        max_supply: u64,
    },
    CreateLiquidity {
        network: Network,
    },
    ActivateDesk {
        desk_account: String,
        owner_account: String,
    },
    DeactivateDesk {
        desk_account: String,
        owner_account: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum AccountOrderResult {
    Client {
        account_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Desk {
        account_id: String,
        market: Market,
        owner_account: String,
        market_url: String,
    },
    DeskActivated {
        desk_account: String,
        owner_account: String,
    },
    DeskDeactivated {
        desk_account: String,
        owner_account: String,
    },
    Faucet {
        account_id: String,
        token_symbol: String,
        decimals: u8,
        max_supply: u64,
    },
    Liquidity {
        account_id: String,
    },
}

impl AccountOrder {
    pub fn kind(&self) -> &'static str {
        match self {
            AccountOrder::CreateClient { .. } => "CreateClientAccount",
            AccountOrder::CreateDesk { .. } => "CreateDeskAccount",
            AccountOrder::CreateFaucet { .. } => "CreateFaucetAccount",
            AccountOrder::CreateLiquidity { .. } => "CreateLiquidityAccount",
            AccountOrder::ActivateDesk { .. } => "ActivateDeskAccount",
            AccountOrder::DeactivateDesk { .. } => "DeactivateDeskAccount",
        }
    }
}

impl AccountOrderResult {
    pub fn kind(&self) -> &'static str {
        match self {
            AccountOrderResult::Client { .. } => "Client",
            AccountOrderResult::Desk { .. } => "Desk",
            AccountOrderResult::DeskActivated { .. } => "DeskActivated",
            AccountOrderResult::DeskDeactivated { .. } => "DeskDeactivated",
            AccountOrderResult::Faucet { .. } => "Faucet",
            AccountOrderResult::Liquidity { .. } => "Liquidity",
        }
    }
}
