use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use currents_core::{WeatherConfig, CacheConfig};

/// Main daemon configuration
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

impl Config {
    pub fn load_from_file() -> Result<Self> {
        let content = currents_core::config::load_config_file()?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse daemon configuration")?;
        Ok(config)
    }
}

