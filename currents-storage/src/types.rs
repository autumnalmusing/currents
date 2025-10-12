use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Configuration for the storage layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to the SQLite database file
    pub database_path: String,
    
    /// Maximum number of days to keep in history (0 = unlimited)
    pub max_history_days: u32,
    
    /// Enable data compression for old records
    pub enable_compression: bool,
    
    /// Compression threshold (days)
    pub compression_threshold: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            database_path: Self::default_database_path(),
            max_history_days: 365,
            enable_compression: true,
            compression_threshold: 30,
        }
    }
}

impl StorageConfig {
    /// Get the default database path
    fn default_database_path() -> String {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("currents").join("weather_storage.db").to_string_lossy().to_string()
        } else {
            "weather_storage.db".to_string()
        }
    }
}

/// Daily aggregated weather data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyWeather {
    pub date: String,
    pub min_temp: f64,
    pub max_temp: f64,
    pub avg_temp: f64,
    pub avg_humidity: f64,
    pub avg_wind_speed: f64,
    pub avg_pressure: Option<f64>,
    pub record_count: i64,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_records: i64,
    pub earliest_timestamp: Option<DateTime<Utc>>,
    pub latest_timestamp: Option<DateTime<Utc>>,
    pub avg_temperature: f64,
    pub min_temperature: f64,
    pub max_temperature: f64,
}