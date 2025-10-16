use mosaic_fi::{Market, note::MosaicNote};
use mosaic_miden::Network;
use rusqlite::{Connection, Result as SqliteResult, params};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Status of a note in the desk
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteStatus {
    New,
    Consumed,
    Invalid,
}

impl NoteStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            NoteStatus::New => "new",
            NoteStatus::Consumed => "consumed",
            NoteStatus::Invalid => "invalid",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "new" => Ok(NoteStatus::New),
            "consumed" => Ok(NoteStatus::Consumed),
            "invalid" => Ok(NoteStatus::Invalid),
            _ => Err(format!("Invalid note status: {}", s)),
        }
    }
}

/// Manages the global desks database at the top level of the project
pub struct DeskStore {
    conn: Connection,
}

/// Manages notes for a specific desk
pub struct DeskNoteStore {
    conn: Connection,
}

impl DeskStore {
    /// Create or open the desk store at the specified path
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path)?;

        // Create the desks table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS desks (
                uuid TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                network TEXT NOT NULL,
                base_code TEXT NOT NULL,
                base_issuer TEXT NOT NULL,
                quote_code TEXT NOT NULL,
                quote_issuer TEXT NOT NULL
            )",
            [],
        )?;

        Ok(DeskStore { conn })
    }

    /// Insert a new desk record
    pub fn insert_desk(
        &self,
        uuid: Uuid,
        path: &Path,
        network: Network,
        market: &Market,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let network_str = match network {
            Network::Testnet => "Testnet",
            Network::Localnet => "Localnet",
        };

        self.conn.execute(
            "INSERT INTO desks (uuid, path, network, base_code, base_issuer, quote_code, quote_issuer) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                uuid.to_string(),
                path.to_str().unwrap(),
                network_str,
                market.base.code,
                market.base.issuer,
                market.quote.code,
                market.quote.issuer
            ],
        )?;

        Ok(())
    }

    /// List all desks
    pub fn list_desks(&self) -> SqliteResult<Vec<(Uuid, PathBuf, Network, Market)>> {
        let mut stmt = self.conn.prepare("SELECT uuid, path, network, base_code, base_issuer, quote_code, quote_issuer FROM desks")?;

        let desks_iter = stmt.query_map([], |row| {
            let uuid_str: String = row.get(0)?;
            let path_str: String = row.get(1)?;
            let network_str: String = row.get(2)?;
            let base_code: String = row.get(3)?;
            let base_issuer: String = row.get(4)?;
            let quote_code: String = row.get(5)?;
            let quote_issuer: String = row.get(6)?;

            let uuid = Uuid::parse_str(&uuid_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            let path = PathBuf::from(path_str);

            let network = match network_str.as_str() {
                "Testnet" => Network::Testnet,
                "Localnet" => Network::Localnet,
                _ => {
                    return Err(rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid network: {}", network_str),
                        )),
                    ));
                }
            };

            let market = Market {
                base: mosaic_fi::Currency {
                    code: base_code,
                    issuer: base_issuer,
                },
                quote: mosaic_fi::Currency {
                    code: quote_code,
                    issuer: quote_issuer,
                },
            };

            Ok((uuid, path, network, market))
        })?;

        let mut desks = Vec::new();
        for desk in desks_iter {
            desks.push(desk?);
        }

        Ok(desks)
    }

    /// Delete a desk by UUID
    pub fn delete_desk(&self, uuid: Uuid) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "DELETE FROM desks WHERE uuid = ?1",
            params![uuid.to_string()],
        )?;
        Ok(())
    }

    /// Get a desk by UUID
    pub fn get_desk(&self, uuid: Uuid) -> SqliteResult<Option<(PathBuf, Network, Market)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT path, network, base_code, base_issuer, quote_code, quote_issuer FROM desks WHERE uuid = ?1")?;

        let result = stmt.query_row(params![uuid.to_string()], |row| {
            let path_str: String = row.get(0)?;
            let network_str: String = row.get(1)?;
            let base_code: String = row.get(2)?;
            let base_issuer: String = row.get(3)?;
            let quote_code: String = row.get(4)?;
            let quote_issuer: String = row.get(5)?;

            let path = PathBuf::from(path_str);

            let network = match network_str.as_str() {
                "Testnet" => Network::Testnet,
                "Localnet" => Network::Localnet,
                _ => {
                    return Err(rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Invalid network: {}", network_str),
                        )),
                    ));
                }
            };

            let market = Market {
                base: mosaic_fi::Currency {
                    code: base_code,
                    issuer: base_issuer,
                },
                quote: mosaic_fi::Currency {
                    code: quote_code,
                    issuer: quote_issuer,
                },
            };

            Ok((path, network, market))
        });

        match result {
            Ok(desk) => Ok(Some(desk)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl DeskNoteStore {
    /// Create or open a desk note store at the specified path
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path)?;

        // Create the notes table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                note_json TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create index on status for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_notes_status ON notes(status)",
            [],
        )?;

        Ok(DeskNoteStore { conn })
    }

    /// Insert a new note
    pub fn insert_note(
        &self,
        note: &MosaicNote,
        status: NoteStatus,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let note_json = serde_json::to_string(note)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO notes (note_json, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![note_json, status.as_str(), now, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Update note status
    pub fn update_note_status(
        &self,
        note_id: i64,
        status: NoteStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            "UPDATE notes SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now, note_id],
        )?;

        Ok(())
    }

    /// Get notes by status
    pub fn get_notes_by_status(
        &self,
        status: NoteStatus,
    ) -> Result<Vec<(i64, MosaicNote)>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, note_json FROM notes WHERE status = ?1 ORDER BY created_at DESC",
        )?;

        let notes_iter = stmt.query_map(params![status.as_str()], |row| {
            let id: i64 = row.get(0)?;
            let note_json: String = row.get(1)?;
            Ok((id, note_json))
        })?;

        let mut notes = Vec::new();
        for note_result in notes_iter {
            let (id, note_json) = note_result?;
            let note: MosaicNote = serde_json::from_str(&note_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            notes.push((id, note));
        }

        Ok(notes)
    }

    /// Get all notes
    pub fn get_all_notes(
        &self,
    ) -> Result<Vec<(i64, MosaicNote, NoteStatus)>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, note_json, status FROM notes ORDER BY created_at DESC")?;

        let notes_iter = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let note_json: String = row.get(1)?;
            let status_str: String = row.get(2)?;
            Ok((id, note_json, status_str))
        })?;

        let mut notes = Vec::new();
        for note_result in notes_iter {
            let (id, note_json, status_str) = note_result?;
            let note: MosaicNote = serde_json::from_str(&note_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            let status = NoteStatus::from_str(&status_str).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
                )
            })?;
            notes.push((id, note, status));
        }

        Ok(notes)
    }

    /// Get a specific note by ID
    pub fn get_note(
        &self,
        note_id: i64,
    ) -> Result<Option<(MosaicNote, NoteStatus)>, Box<dyn std::error::Error>> {
        let mut stmt = self
            .conn
            .prepare("SELECT note_json, status FROM notes WHERE id = ?1")?;

        let result = stmt.query_row(params![note_id], |row| {
            let note_json: String = row.get(0)?;
            let status_str: String = row.get(1)?;
            Ok((note_json, status_str))
        });

        match result {
            Ok((note_json, status_str)) => {
                let note: MosaicNote = serde_json::from_str(&note_json)?;
                let status = NoteStatus::from_str(&status_str)?;
                Ok(Some((note, status)))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }
}
