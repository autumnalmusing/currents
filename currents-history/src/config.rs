use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;
use dirs;

/// Threshold-based collection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSettings {
    /// Temperature change threshold (degrees)
    pub temperature_threshold: f64,
    
    /// Humidity change threshold (percentage)
    pub humidity_threshold: f64,
    
    /// Wind speed change threshold (m/s)
    pub wind_speed_threshold: f64,
    
    /// Pressure change threshold (hPa)
    pub pressure_threshold: f64,
    
    /// Weather description change (record if description changes)
    pub track_description_changes: bool,
    
    /// Minimum time between collections (seconds)
    pub min_interval: u64,
    
    /// Maximum time between collections (seconds)
    pub max_interval: u64,
    
    /// Force collection at regular intervals even if no thresholds crossed
    pub force_interval: Option<u64>,
}

/// Configuration for the history module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Path to the SQLite database file
    pub database_path: String,
    
    /// Maximum number of days to keep in history (0 = unlimited)
    pub max_history_days: u32,
    
    /// Enable automatic data collection from daemon
    pub auto_collect: bool,
    
    /// Data collection interval in seconds
    pub collection_interval: u64,
    
    /// Collection strategy: "interval", "threshold", "adaptive", or "hybrid"
    pub collection_strategy: String,
    
    /// Threshold-based collection settings
    pub threshold_settings: Option<ThresholdSettings>,
    
    /// Enable data compression for old records
    pub enable_compression: bool,
    
    /// Compression threshold (days)
    pub compression_threshold: u32,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            database_path: Self::default_database_path(),
            max_history_days: 365, // Keep 1 year by default
            auto_collect: true,
            collection_interval: 3600, // 1 hour
            collection_strategy: "interval".to_string(),
            threshold_settings: None,
            enable_compression: true,
            compression_threshold: 30, // Compress data older than 30 days
        }
    }
}

impl HistoryConfig {
    /// Get the default database path
    fn default_database_path() -> String {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("currents").join("weather_history.db").to_string_lossy().to_string()
        } else {
            "weather_history.db".to_string()
        }
    }
    
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read configuration file")?;
        
        let config: Self = toml::from_str(&content)
            .context("Failed to parse configuration file")?;
        
        // Ensure database directory exists
        if let Some(db_dir) = std::path::Path::new(&config.database_path).parent() {
            std::fs::create_dir_all(db_dir)
                .context("Failed to create database directory")?;
        }
        
        Ok(config)
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .context("Failed to serialize configuration")?;
        
        std::fs::write(path, content)
            .context("Failed to write configuration file")?;
        
        Ok(())
    }
    
    /// Merge with existing configuration (for updates)
    pub fn merge_with(&mut self, other: &HistoryConfig) {
        if !other.database_path.is_empty() {
            self.database_path = other.database_path.clone();
        }
        if other.max_history_days > 0 {
            self.max_history_days = other.max_history_days;
        }
        self.auto_collect = other.auto_collect;
        if other.collection_interval > 0 {
            self.collection_interval = other.collection_interval;
        }
        self.enable_compression = other.enable_compression;
        if other.compression_threshold > 0 {
            self.compression_threshold = other.compression_threshold;
        }
    }
}

/// History-specific configuration section for the main config file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySection {
    /// Enable history tracking
    pub enabled: Option<bool>,
    
    /// Database path
    pub database_path: Option<String>,
    
    /// Maximum history days
    pub max_history_days: Option<u32>,
    
    /// Auto collect data
    pub auto_collect: Option<bool>,
    
    /// Collection interval (seconds)
    pub collection_interval: Option<u64>,
    
    /// Enable compression
    pub enable_compression: Option<bool>,
    
    /// Compression threshold (days)
    pub compression_threshold: Option<u32>,
}

impl Default for HistorySection {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            database_path: None,
            max_history_days: Some(365),
            auto_collect: Some(true),
            collection_interval: Some(3600),
            enable_compression: Some(true),
            compression_threshold: Some(30),
        }
    }
}

impl From<HistorySection> for HistoryConfig {
    fn from(section: HistorySection) -> Self {
        Self {
            database_path: section.database_path.unwrap_or_else(Self::default_database_path),
            max_history_days: section.max_history_days.unwrap_or(365),
            auto_collect: section.auto_collect.unwrap_or(true),
            collection_interval: section.collection_interval.unwrap_or(3600),
            collection_strategy: "interval".to_string(),
            threshold_settings: None,
            enable_compression: section.enable_compression.unwrap_or(true),
            compression_threshold: section.compression_threshold.unwrap_or(30),
        }
    }
}