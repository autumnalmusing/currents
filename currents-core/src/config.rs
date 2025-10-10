use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use anyhow::{Result, Context};

/// Core weather API configuration - shared across all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    pub api_key: String,
    pub location: String,
    pub units: String, // "metric", "imperial", "kelvin"
    pub provider: String, // "openweathermap", "weatherapi", etc.
    /// Maximum API calls allowed per day (default: 1000)
    #[serde(default = "default_api_limit")]
    pub api_daily_limit: u64,
}

fn default_api_limit() -> u64 {
    1000
}

/// Cache configuration - shared across all tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Path to cached weather snapshot (Waybar and tools read from here)
    pub path: String,
    /// Time-to-live for cached data in seconds
    pub ttl_seconds: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        let default_path = format!(
            "{}/.cache/currents/weather.json",
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
        );
        Self {
            path: default_path,
            ttl_seconds: 300,
        }
    }
}

/// Helper to get standard config file path
pub fn get_config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("currents").join("config.toml")
}

/// Helper to load config file contents
pub fn load_config_file() -> Result<String> {
    let config_path = get_config_path();
    fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {:?}", config_path))
}

