use crate::Network;
use rusqlite::{Connection, OptionalExtension, Result as SqliteResult, ffi, params};
use std::path::Path;

/// Account and asset storage using SQLite
pub struct Store {
    conn: Connection,
}

#[derive(Debug, Clone)]
pub struct AssetRecord {
    pub symbol: String,
    pub account: String,
    pub decimals: u8,
    pub max_supply: Option<String>,
    pub owned: bool,
}

impl Store {
    /// Create or open a store at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let mut conn = Connection::open(path)?;

        // Enforce foreign key constraints and ensure schema exists
        conn.pragma_update(None, "foreign_keys", &true)?;
        ensure_schema(&mut conn)?;

        Ok(Store { conn })
    }

    /// Insert a new account with an optional display name
    pub fn insert_account(
        &self,
        account_id: &str,
        network: Network,
        account_type: &str,
        name: Option<&str>,
    ) -> SqliteResult<()> {
        let network_str = match network {
            Network::Testnet => "Testnet",
            Network::Localnet => "Localnet",
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO accounts (id, network, typ, name) VALUES (?1, ?2, ?3, ?4)",
            params![account_id, network_str, account_type, name],
        )?;

        Ok(())
    }

    /// List all accounts
    pub fn list_accounts(&self) -> SqliteResult<Vec<(String, String, String, Option<String>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, network, typ, name FROM accounts ORDER BY id")?;

        let accounts = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(accounts)
    }

    /// List accounts for a specific network
    pub fn list_accounts_by_network(
        &self,
        network: Network,
    ) -> SqliteResult<Vec<(String, String, Option<String>)>> {
        let network_str = match network {
            Network::Testnet => "Testnet",
            Network::Localnet => "Localnet",
        };

        let mut stmt = self
            .conn
            .prepare("SELECT id, typ, name FROM accounts WHERE network = ?1 ORDER BY id")?;

        let accounts = stmt
            .query_map([network_str], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(accounts)
    }

    /// Delete an account
    pub fn delete_account(&self, account_id: &str) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM accounts WHERE id = ?1", params![account_id])?;
        Ok(())
    }

    /// Delete all accounts
    pub fn delete_all_accounts(&self) -> SqliteResult<()> {
        self.conn.execute("DELETE FROM accounts", [])?;
        Ok(())
    }

    /// Check whether an account exists in the store.
    pub fn has_account(&self, account_id: &str) -> SqliteResult<bool> {
        let mut stmt = self
            .conn
            .prepare("SELECT 1 FROM accounts WHERE id = ?1 LIMIT 1")?;
        let exists: Option<i64> = stmt
            .query_row(params![account_id], |row| row.get(0))
            .optional()?;
        Ok(exists.is_some())
    }

    /// Insert or update an asset entry for the user.
    pub fn upsert_asset(&self, asset: &AssetRecord) -> SqliteResult<()> {
        if asset.owned && !self.has_account(&asset.account)? {
            return Err(rusqlite::Error::SqliteFailure(
                ffi::Error::new(ffi::SQLITE_CONSTRAINT),
                Some("Owned assets must reference an existing account".to_string()),
            ));
        }

        self.conn.execute(
            "INSERT INTO assets (symbol, account, decimals, max_supply, owned)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(symbol, account) DO UPDATE SET
                decimals = excluded.decimals,
                max_supply = excluded.max_supply,
                owned = excluded.owned",
            params![
                asset.symbol,
                asset.account,
                asset.decimals as i64,
                asset.max_supply.as_deref(),
                if asset.owned { 1 } else { 0 },
            ],
        )?;

        Ok(())
    }

    /// List all assets stored in the database.
    pub fn list_assets(&self) -> SqliteResult<Vec<AssetRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT symbol, account, decimals, max_supply, owned
             FROM assets
             ORDER BY created_at ASC",
        )?;

        let assets = stmt
            .query_map([], |row| {
                Ok(AssetRecord {
                    symbol: row.get(0)?,
                    account: row.get(1)?,
                    decimals: row.get::<_, i64>(2)? as u8,
                    max_supply: row.get(3)?,
                    owned: row.get::<_, i64>(4)? != 0,
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(assets)
    }

    /// Remove an asset entry.
    pub fn delete_asset(&self, symbol: &str, account: &str) -> SqliteResult<()> {
        self.conn.execute(
            "DELETE FROM assets WHERE symbol = ?1 AND account = ?2",
            params![symbol, account],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_operations() {
        let store = Store::new(":memory:").unwrap();

        // Insert accounts
        store
            .insert_account(
                "test_account_1",
                Network::Testnet,
                "Client",
                Some("Primary"),
            )
            .unwrap();
        store
            .insert_account("test_account_2", Network::Localnet, "Desk", None)
            .unwrap();

        // List all accounts
        let accounts = store.list_accounts().unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].3.as_deref(), Some("Primary"));

        // List by network
        let testnet_accounts = store.list_accounts_by_network(Network::Testnet).unwrap();
        assert_eq!(testnet_accounts.len(), 1);
        assert_eq!(testnet_accounts[0].0, "test_account_1");
        assert_eq!(testnet_accounts[0].2.as_deref(), Some("Primary"));

        // Delete an account
        store.delete_account("test_account_1").unwrap();
        let accounts = store.list_accounts().unwrap();
        assert_eq!(accounts.len(), 1);

        // Delete all accounts
        store.delete_all_accounts().unwrap();
        let accounts = store.list_accounts().unwrap();
        assert_eq!(accounts.len(), 0);
    }

    #[test]
    fn test_asset_operations() {
        let store = Store::new(":memory:").unwrap();

        // Linked asset without owning account should succeed
        let linked = AssetRecord {
            symbol: "USDC".to_string(),
            account: "mtst_linked".to_string(),
            decimals: 6,
            max_supply: Some("0".to_string()),
            owned: false,
        };
        store.upsert_asset(&linked).unwrap();

        // Owning asset without corresponding account should fail due to FK
        let owned_missing_account = AssetRecord {
            symbol: "MID".to_string(),
            account: "mtst_owned".to_string(),
            decimals: 8,
            max_supply: Some("1000000".to_string()),
            owned: true,
        };
        match store.upsert_asset(&owned_missing_account).unwrap_err() {
            rusqlite::Error::SqliteFailure(err, Some(_))
                if err.code == ffi::ErrorCode::ConstraintViolation => {}
            err => panic!("Unexpected error: {err}"),
        }

        // Once the account exists, insert succeeds
        store
            .insert_account("mtst_owned", Network::Testnet, "Faucet", Some("MID faucet"))
            .unwrap();
        store.upsert_asset(&owned_missing_account).unwrap();

        // List assets
        let assets = store.list_assets().unwrap();
        assert_eq!(assets.len(), 2);
        assert!(
            assets
                .iter()
                .any(|asset| asset.symbol == "USDC" && !asset.owned)
        );
        assert!(
            assets
                .iter()
                .any(|asset| asset.symbol == "MID" && asset.owned)
        );

        // Delete asset
        store.delete_asset("USDC", "mtst_linked").unwrap();
        let assets = store.list_assets().unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].symbol, "MID");
    }
}

fn ensure_schema(conn: &mut Connection) -> SqliteResult<()> {
    let tx = conn.transaction()?;
    ensure_accounts_table(&tx)?;
    ensure_assets_table(&tx)?;
    tx.commit()
}

fn ensure_accounts_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS accounts (
            id TEXT PRIMARY KEY,
            network TEXT NOT NULL,
            typ TEXT NOT NULL,
            name TEXT
        )",
        [],
    )?;

    // Ensure the 'name' column exists for databases created before the column was added.
    let mut stmt = conn.prepare("PRAGMA table_info(accounts)")?;
    let mut rows = stmt.query([])?;
    let mut has_name_column = false;

    while let Some(row) = rows.next()? {
        let column_name: String = row.get(1)?;
        if column_name == "name" {
            has_name_column = true;
            break;
        }
    }

    if !has_name_column {
        // Existing databases created before the 'name' column may not have it.
        // Ignore errors since the column can already exist when multiple callers race.
        let _ = conn.execute("ALTER TABLE accounts ADD COLUMN name TEXT", []);
    }

    Ok(())
}

fn ensure_assets_table(conn: &Connection) -> SqliteResult<()> {
    if !table_exists(conn, "assets")? {
        create_assets_table(conn)?;
        return Ok(());
    }

    // Determine whether the existing table still has a foreign key constraint that needs removal.
    if assets_has_foreign_key(conn)? {
        conn.execute("DROP TABLE IF EXISTS assets_legacy", [])?;
        conn.execute("ALTER TABLE assets RENAME TO assets_legacy", [])?;

        create_assets_table(conn)?;

        conn.execute(
            "INSERT INTO assets (symbol, account, decimals, max_supply, owned, created_at)
             SELECT symbol,
                    account,
                    decimals,
                    max_supply,
                    COALESCE(owned, 0),
                    created_at
             FROM assets_legacy",
            [],
        )?;

        conn.execute("DROP TABLE assets_legacy", [])?;
    }

    // Ensure the supporting index exists.
    conn.execute(
        "CREATE INDEX IF NOT EXISTS assets_account_idx ON assets(account)",
        [],
    )?;

    Ok(())
}

fn create_assets_table(conn: &Connection) -> SqliteResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS assets (
            symbol TEXT NOT NULL,
            account TEXT NOT NULL,
            decimals INTEGER NOT NULL,
            max_supply TEXT,
            owned INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY(symbol, account)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS assets_account_idx ON assets(account)",
        [],
    )?;

    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> SqliteResult<bool> {
    let mut stmt =
        conn.prepare("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1")?;
    let exists: Option<i64> = stmt.query_row(params![name], |row| row.get(0)).optional()?;
    Ok(exists.is_some())
}

fn assets_has_foreign_key(conn: &Connection) -> SqliteResult<bool> {
    let mut stmt = conn.prepare("PRAGMA foreign_key_list(assets)")?;
    let mut rows = stmt.query([])?;
    Ok(rows.next()?.is_some())
}
