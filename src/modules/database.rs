use anyhow::Result;
use rusqlite::{params, Connection};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute_batch("
            PRAGMA journal_mode = WAL;

            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            CREATE TABLE IF NOT EXISTS blocks (
                height INTEGER PRIMARY KEY,
                tx_count INTEGER,
                processed_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS claims (
                inscription_id TEXT PRIMARY KEY,
                block_height INTEGER,
                content TEXT,
                parsed_type TEXT,
                status TEXT,
                reason TEXT
            );

            CREATE TABLE IF NOT EXISTS districts (
                number INTEGER PRIMARY KEY,
                inscription_id TEXT,
                block_height INTEGER
            );

            CREATE TABLE IF NOT EXISTS parcels (
                composite_id TEXT PRIMARY KEY,
                number INTEGER,
                block_number INTEGER,
                inscription_id TEXT,
                parent_district_number INTEGER
            );
        ")?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn set_last_block(&self, height: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('last_block', ?1)",
            params![height.to_string()],
        )?;
        Ok(())
    }

    pub fn get_last_block(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = 'last_block'")?;
        let res = stmt.query_row([], |row| {
            let val: String = row.get(0)?;
            Ok(val.parse::<u64>().unwrap_or(0))
        });
        match res {
            Ok(height) => Ok(height),
            Err(_) => Ok(0),
        }
    }

    pub fn save_district(&self, number: u64, id: &str, height: u64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute(
            "INSERT OR IGNORE INTO districts (number, inscription_id, block_height) VALUES (?1, ?2, ?3)",
            params![number, id, height],
        )?;
        Ok(rows > 0)
    }

    pub fn get_district(&self, number: u64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT inscription_id FROM districts WHERE number = ?1"
        )?;
        let res = stmt.query_row(params![number], |row| row.get(0));
        match res {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_parcels(&self, district: u64) -> Result<Vec<(u64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT number, inscription_id FROM parcels WHERE parent_district_number = ?1 ORDER BY number ASC"
        )?;
        let rows = stmt.query_map(params![district], |row| {
            Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut parcels = Vec::new();
        for row in rows {
            parcels.push(row?);
        }
        Ok(parcels)
    }
}