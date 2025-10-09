use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub weather: WeatherConfig,
    pub alerts: Vec<AlertRule>,
    pub notifications: NotificationConfig,
    pub polling: PollingConfig,
    #[serde(default)]
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    pub api_key: String,
    pub location: String,
    pub units: String, // "metric", "imperial", "kelvin"
    pub provider: String, // "openweathermap", "weatherapi", etc.
    /// Maximum API calls allowed per day (default: 1000)
    /// Increase this if you're willing to pay for more API calls
    #[serde(default = "default_api_limit")]
    pub api_daily_limit: u64,
}

fn default_api_limit() -> u64 {
    1000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub condition: WeatherCondition,
    pub message: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherCondition {
    pub temperature: Option<TemperatureRange>,
    pub humidity: Option<Range>,
    pub wind_speed: Option<Range>,
    pub precipitation: Option<PrecipitationCondition>,
    pub description: Option<String>, // Weather description keywords
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecipitationCondition {
    pub intensity: Option<String>, // "light", "moderate", "heavy"
    pub probability: Option<f64>, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub urgency: String, // "low", "normal", "critical"
    pub timeout: u32, // milliseconds
    pub sound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollingConfig {
    pub interval_seconds: u64,
    pub retry_attempts: u32,
    pub retry_delay_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Path to cached weather snapshot (Waybar reads from here)
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

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        
        // Determine format by file extension
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("toml");
        
        
        match extension {
            "toml" => Self::load_toml(&content),
            "kdl" => Self::load_kdl(&content),
            _ => {
                // Default to TOML if extension is unknown
                Self::load_toml(&content)
            }
        }
    }
    
    fn load_toml(content: &str) -> Result<Self> {
        let config: Config = toml::from_str(content)
            .with_context(|| "Failed to parse TOML configuration")?;
        Ok(config)
    }
    
    fn load_kdl(_content: &str) -> Result<Self> {
        // TODO: Implement KDL parsing
        // For now, return an error indicating KDL support is not yet implemented
        Err(anyhow::anyhow!("KDL support is not yet implemented. Please use TOML format for now."))
    }
}
