use crate::Network;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;

/// Account storage using SQLite
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Create or open a store at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;

        // Create accounts table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                network TEXT NOT NULL,
                typ TEXT NOT NULL
            )",
            [],
        )?;

        Ok(Store { conn })
    }

    /// Insert a new account
    pub fn insert_account(
        &self,
        account_id: &str,
        network: Network,
        account_type: &str,
    ) -> SqliteResult<()> {
        let network_str = match network {
            Network::Testnet => "testnet",
            Network::Localnet => "localnet",
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO accounts (id, network, typ) VALUES (?1, ?2, ?3)",
            params![account_id, network_str, account_type],
        )?;

        Ok(())
    }

    /// List all accounts
    pub fn list_accounts(&self) -> SqliteResult<Vec<(String, String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, network, typ FROM accounts ORDER BY id")?;

        let accounts = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(accounts)
    }

    /// List accounts for a specific network
    pub fn list_accounts_by_network(&self, network: Network) -> SqliteResult<Vec<(String, String)>> {
        let network_str = match network {
            Network::Testnet => "testnet",
            Network::Localnet => "localnet",
        };

        let mut stmt = self
            .conn
            .prepare("SELECT id, typ FROM accounts WHERE network = ?1 ORDER BY id")?;

        let accounts = stmt
            .query_map([network_str], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_operations() {
        let store = Store::new(":memory:").unwrap();

        // Insert accounts
        store
            .insert_account("test_account_1", Network::Testnet, "Client")
            .unwrap();
        store
            .insert_account("test_account_2", Network::Localnet, "Desk")
            .unwrap();

        // List all accounts
        let accounts = store.list_accounts().unwrap();
        assert_eq!(accounts.len(), 2);

        // List by network
        let testnet_accounts = store.list_accounts_by_network(Network::Testnet).unwrap();
        assert_eq!(testnet_accounts.len(), 1);
        assert_eq!(testnet_accounts[0].0, "test_account_1");

        // Delete an account
        store.delete_account("test_account_1").unwrap();
        let accounts = store.list_accounts().unwrap();
        assert_eq!(accounts.len(), 1);

        // Delete all accounts
        store.delete_all_accounts().unwrap();
        let accounts = store.list_accounts().unwrap();
        assert_eq!(accounts.len(), 0);
    }
}
