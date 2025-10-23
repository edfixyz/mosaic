use crate::Network;
use rusqlite::{Connection, Result as SqliteResult, params};
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
        ensure_account_table(&conn)?;

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
}

fn ensure_account_table(conn: &Connection) -> SqliteResult<()> {
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
