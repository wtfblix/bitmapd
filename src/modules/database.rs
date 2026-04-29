use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens connection and initializes tables
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        // Initialize Schema
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            
            -- Tracks sync progress
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            -- Blocks we have fully processed
            CREATE TABLE IF NOT EXISTS blocks (
                height INTEGER PRIMARY KEY,
                tx_count INTEGER,
                processed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            -- All raw inscription claims we find
            CREATE TABLE IF NOT EXISTS claims (
                inscription_id TEXT PRIMARY KEY,
                block_height INTEGER,
                content TEXT,
                parsed_type TEXT,
                status TEXT, -- 'accepted', 'rejected'
                reason TEXT
            );

            -- Valid Districts
            CREATE TABLE IF NOT EXISTS districts (
                number INTEGER PRIMARY KEY,
                inscription_id TEXT,
                block_height INTEGER
            );

            -- Valid Parcels
            CREATE TABLE IF NOT EXISTS parcels (
                composite_id TEXT PRIMARY KEY, -- index.block
                number INTEGER,
                block_number INTEGER,
                inscription_id TEXT,
                parent_district_number INTEGER
            );
        ")?;

        Ok(Self { conn })
    }

    /// Sets the last processed block height
    pub fn set_last_block(&self, height: u64) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('last_block', ?1)",
            params![height.to_string()],
        )?;
        Ok(())
    }

    /// Gets the last processed block height for resuming
    pub fn get_last_block(&self) -> Result<u64> {
        let mut stmt = self.conn.prepare("SELECT value FROM meta WHERE key = 'last_block'")?;
        let res = stmt.query_row([], |row| {
            let val: String = row.get(0)?;
            Ok(val.parse::<u64>().unwrap_or(0))
        });

        match res {
            Ok(height) => Ok(height),
            Err(_) => Ok(0),
        }
    }

    /// Saves a newly discovered District (if first claim)
    pub fn save_district(&self, number: u64, id: &str, height: u64) -> Result<bool> {
        // First claim wins logic: INSERT OR IGNORE
        let rows = self.conn.execute(
            "INSERT OR IGNORE INTO districts (number, inscription_id, block_height) VALUES (?1, ?2, ?3)",
            params![number, id, height],
        )?;
        
        Ok(rows > 0) // Returns true if this was actually inserted (first claim)
    }
}