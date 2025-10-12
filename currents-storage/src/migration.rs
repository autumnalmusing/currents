use anyhow::{Result, Context};
use rusqlite::{Connection, params};

/// Database migration manager
pub struct MigrationManager<'a> {
    conn: &'a Connection,
}

impl<'a> MigrationManager<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    
    /// Run all pending migrations
    pub fn migrate(&self) -> Result<()> {
        self.create_migrations_table()?;
        self.run_migration(1, "create_weather_history_table", Self::migration_001_create_weather_history_table)?;
        self.run_migration(2, "add_indexes", Self::migration_002_add_indexes)?;
        self.run_migration(3, "add_compression_support", Self::migration_003_add_compression_support)?;
        self.run_migration(4, "add_location_support", Self::migration_004_add_location_support)?;
        Ok(())
    }
    
    /// Create migrations tracking table
    fn create_migrations_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        ).context("Failed to create migrations table")?;
        Ok(())
    }
    
    /// Run a specific migration if not already applied
    fn run_migration<F>(&self, version: i32, name: &str, migration_fn: F) -> Result<()>
    where
        F: Fn(&Connection) -> Result<()>,
    {
        // Check if migration already applied
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM migrations WHERE version = ?1")?;
        let count: i32 = stmt.query_row(params![version], |row| row.get(0))?;
        
        if count > 0 {
            return Ok(()); // Migration already applied
        }
        
        // Run migration
        migration_fn(&self.conn)?;
        
        // Record migration
        self.conn.execute(
            "INSERT INTO migrations (version, name) VALUES (?1, ?2)",
            params![version, name],
        ).context("Failed to record migration")?;
        
        Ok(())
    }
    
    /// Migration 001: Create weather history table
    fn migration_001_create_weather_history_table(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS weather_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp INTEGER NOT NULL,
                temperature REAL NOT NULL,
                humidity REAL NOT NULL,
                wind_speed REAL NOT NULL,
                wind_direction TEXT,
                pressure REAL,
                visibility REAL,
                uv_index REAL,
                cloud_cover REAL,
                aqi REAL,
                description TEXT NOT NULL,
                precipitation_intensity TEXT,
                precipitation_probability REAL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        ).context("Failed to create weather_history table")?;
        Ok(())
    }
    
    /// Migration 002: Add indexes for performance
    fn migration_002_add_indexes(conn: &Connection) -> Result<()> {
        // Create index for efficient time-based queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON weather_history(timestamp)",
            [],
        ).context("Failed to create timestamp index")?;
        
        // Create index for efficient metric queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_temperature ON weather_history(temperature)",
            [],
        ).context("Failed to create temperature index")?;
        
        Ok(())
    }
    
    /// Migration 003: Add compression support
    fn migration_003_add_compression_support(conn: &Connection) -> Result<()> {
        // Add compression metadata table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS compression_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                compressed_date TEXT NOT NULL,
                original_records INTEGER NOT NULL,
                compressed_records INTEGER NOT NULL,
                compression_ratio REAL NOT NULL,
                created_at INTEGER DEFAULT (strftime('%s', 'now'))
            )",
            [],
        ).context("Failed to create compression_metadata table")?;
        
        Ok(())
    }
    
    /// Migration 004: Add location support
    fn migration_004_add_location_support(conn: &Connection) -> Result<()> {
        // Add location_id column to weather_history table
        // Check if column already exists first
        let has_location_col: bool = {
            let mut stmt = conn.prepare("PRAGMA table_info(weather_history)")?;
            let mut rows = stmt.query([])?;
            let mut found = false;
            while let Some(row) = rows.next()? {
                let col_name: String = row.get(1)?;
                if col_name == "location_id" {
                    found = true;
                    break;
                }
            }
            found
        };
        
        if !has_location_col {
            conn.execute(
                "ALTER TABLE weather_history ADD COLUMN location_id TEXT DEFAULT 'default'",
                [],
            ).context("Failed to add location_id column")?;
            
            // Create index for efficient location-based queries
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_location_id ON weather_history(location_id)",
                [],
            ).context("Failed to create location_id index")?;
            
            // Create composite index for location + timestamp queries
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_location_timestamp ON weather_history(location_id, timestamp)",
                [],
            ).context("Failed to create location_timestamp index")?;
        }
        
        Ok(())
    }
}