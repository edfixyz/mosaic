use rusqlite::{Connection, Result, params};
use serde::Serialize;
use std::collections::HashMap;

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

pub struct NewAsset<'a> {
    pub symbol: &'a str,
    pub account: &'a str,
    pub max_supply: &'a str,
    pub decimals: u8,
    pub verified: bool,
    pub owner: bool,
    pub hidden: bool,
}

pub struct AssetStore {
    conn: Connection,
}

impl AssetStore {
    pub fn open<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.ensure_schema()?;
        Ok(store)
    }

    fn ensure_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS assets (
                user_secret TEXT NOT NULL,
                account TEXT NOT NULL PRIMARY KEY,
                symbol TEXT NOT NULL,
                max_supply TEXT NOT NULL,
                decimals INTEGER NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                owner INTEGER NOT NULL DEFAULT 0,
                hidden INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        let _ = self.conn.execute(
            "ALTER TABLE assets ADD COLUMN owner INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE assets ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0",
            [],
        );

        self.conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS assets_account_unique ON assets(account)",
            [],
        )?;

        Ok(())
    }

    pub fn ensure_default_asset(&self, user_secret: &str) -> Result<()> {
        let asset = NewAsset {
            symbol: "BTC",
            account: "mtst1qrkc5sp34wkncgr9tp9ghjsjv9cqq0u8da0",
            max_supply: "2100000000000000",
            decimals: 8,
            verified: true,
            owner: false,
            hidden: false,
        };
        self.insert_asset(user_secret, &asset)
    }

    pub fn insert_asset(&self, user_secret: &str, asset: &NewAsset<'_>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO assets (user_secret, account, symbol, max_supply, decimals, verified, owner, hidden)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(account) DO UPDATE SET
                symbol = excluded.symbol,
                max_supply = excluded.max_supply,
                decimals = excluded.decimals,
                verified = excluded.verified,
                hidden = excluded.hidden,
                owner = CASE WHEN excluded.owner = 1 THEN 1 ELSE assets.owner END,
                user_secret = CASE WHEN excluded.owner = 1 THEN excluded.user_secret ELSE assets.user_secret END",
            params![
                user_secret,
                asset.account,
                asset.symbol,
                asset.max_supply,
                asset.decimals as i32,
                if asset.verified { 1 } else { 0 },
                if asset.owner { 1 } else { 0 },
                if asset.hidden { 1 } else { 0 },
            ],
        )?;
        Ok(())
    }

    fn list_assets_for_key(&self, key: &str) -> Result<Vec<StoredAsset>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, account, max_supply, decimals, verified, owner, hidden FROM assets
             WHERE hidden = 0 AND user_secret = ?1
             ORDER BY created_at ASC",
        )?;

        let items = stmt
            .query_map(params![key], |row| {
                Ok(StoredAsset {
                    symbol: row.get(0)?,
                    account: row.get(1)?,
                    max_supply: row.get(2)?,
                    decimals: row.get::<_, i64>(3)? as u8,
                    verified: row.get::<_, i64>(4)? != 0,
                    owner: row.get::<_, i64>(5)? != 0,
                    hidden: row.get::<_, i64>(6)? != 0,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(items)
    }

    pub fn list_assets_for_user(&self, user_secret: &str) -> Result<Vec<StoredAsset>> {
        let mut assets_map: HashMap<String, StoredAsset> = HashMap::new();

        for key in [&"__default__", &user_secret] {
            for asset in self.list_assets_for_key(key)? {
                assets_map.insert(asset.account.clone(), asset);
            }
        }

        Ok(assets_map.into_values().collect())
    }
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
