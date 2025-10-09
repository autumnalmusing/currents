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
    #[serde(default)]
    pub forecast_highlights: ForecastHighlights,
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
    /// How often to repeat the alert: "once" (default), "always", or duration in seconds
    #[serde(default = "default_alert_repeat")]
    pub repeat: String,
}

fn default_alert_repeat() -> String {
    "once".to_string()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastHighlights {
    /// Temperature thresholds (in configured units)
    #[serde(default)]
    pub temperature: TemperatureHighlights,
    /// Humidity thresholds (percentage)
    #[serde(default)]
    pub humidity: ValueHighlights,
    /// Wind speed thresholds (in configured units)
    #[serde(default)]
    pub wind_speed: ValueHighlights,
    /// Precipitation probability thresholds (percentage)
    #[serde(default)]
    pub precipitation: ValueHighlights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureHighlights {
    /// Temperature above this is highlighted
    pub high: Option<f64>,
    /// Color for high temperature (default: "red")
    /// Options: "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"
    #[serde(default = "default_high_temp_color")]
    pub high_color: String,
    /// Temperature below this is highlighted
    pub low: Option<f64>,
    /// Color for low temperature (default: "blue")
    #[serde(default = "default_low_temp_color")]
    pub low_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueHighlights {
    /// Value above this is highlighted
    pub high: Option<f64>,
    /// Color for high value (default: "red")
    /// Options: "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"
    #[serde(default = "default_high_color")]
    pub high_color: String,
    /// Value below this is highlighted
    pub low: Option<f64>,
    /// Color for low value (default: "yellow")
    #[serde(default = "default_low_color")]
    pub low_color: String,
}

fn default_high_temp_color() -> String {
    "red".to_string()
}

fn default_low_temp_color() -> String {
    "blue".to_string()
}

fn default_high_color() -> String {
    "red".to_string()
}

fn default_low_color() -> String {
    "yellow".to_string()
}

impl Default for ForecastHighlights {
    fn default() -> Self {
        Self {
            temperature: TemperatureHighlights::default(),
            humidity: ValueHighlights::default(),
            wind_speed: ValueHighlights::default(),
            precipitation: ValueHighlights::default(),
        }
    }
}

impl Default for TemperatureHighlights {
    fn default() -> Self {
        Self {
            high: Some(30.0), // 30°C / 86°F
            high_color: "red".to_string(),
            low: Some(0.0),   // 0°C / 32°F
            low_color: "blue".to_string(),
        }
    }
}

impl Default for ValueHighlights {
    fn default() -> Self {
        Self {
            high: None,
            high_color: "red".to_string(),
            low: None,
            low_color: "yellow".to_string(),
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
