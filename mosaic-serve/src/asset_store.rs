use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StoredAsset {
    pub symbol: String,
    pub account: String,
    #[serde(rename = "maxSupply")]
    pub max_supply: String,
    pub decimals: u8,
    pub verified: bool,
    pub owner: bool,
    pub hidden: bool,
}

pub fn default_assets() -> Vec<StoredAsset> {
    vec![StoredAsset {
        account: "mtst1qrkc5sp34wkncgr9tp9ghjsjv9cqq0u8da0".to_string(),
        symbol: "BTC".to_string(),
        max_supply: "2100000000000000".to_string(),
        decimals: 8,
        verified: true,
        owner: false,
        hidden: false,
    }]
}
