use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use currents_core::WeatherConfig;

/// Forecast tool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub weather: WeatherConfig,
    #[serde(default)]
    pub forecast_highlights: ForecastHighlights,
    #[serde(default)]
    pub forecast_display: ForecastDisplayConfig,
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
    /// Pressure thresholds (in hPa/mb)
    #[serde(default)]
    pub pressure: ValueHighlights,
    /// Visibility thresholds (in km)
    #[serde(default)]
    pub visibility: ValueHighlights,
    /// UV Index thresholds
    #[serde(default)]
    pub uv_index: ValueHighlights,
    /// Cloud cover thresholds (percentage)
    #[serde(default)]
    pub cloud_cover: ValueHighlights,
    /// Air Quality Index thresholds (US EPA 1-6 scale)
    #[serde(default)]
    pub aqi: ValueHighlights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDisplayConfig {
    /// Show date column (always shown)
    #[serde(default = "default_true")]
    pub show_date: bool,
    /// Show weather description
    #[serde(default = "default_true")]
    pub show_weather: bool,
    /// Show temperature range
    #[serde(default = "default_true")]
    pub show_temp: bool,
    /// Show humidity
    #[serde(default = "default_true")]
    pub show_humidity: bool,
    /// Show wind speed
    #[serde(default = "default_true")]
    pub show_wind: bool,
    /// Show precipitation probability
    #[serde(default = "default_true")]
    pub show_precip: bool,
    /// Show atmospheric pressure
    #[serde(default = "default_false")]
    pub show_pressure: bool,
    /// Show visibility
    #[serde(default = "default_false")]
    pub show_visibility: bool,
    /// Show UV index
    #[serde(default = "default_false")]
    pub show_uv: bool,
    /// Show cloud cover
    #[serde(default = "default_false")]
    pub show_clouds: bool,
    /// Show wind direction
    #[serde(default = "default_false")]
    pub show_wind_dir: bool,
    /// Show air quality index
    #[serde(default = "default_false")]
    pub show_aqi: bool,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureHighlights {
    /// Temperature above this is highlighted
    pub high: Option<f64>,
    /// Color for high temperature (default: "red")
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
            pressure: ValueHighlights::default(),
            visibility: ValueHighlights::default(),
            uv_index: ValueHighlights::default(),
            cloud_cover: ValueHighlights::default(),
            aqi: ValueHighlights::default(),
        }
    }
}

impl Default for ForecastDisplayConfig {
    fn default() -> Self {
        Self {
            show_date: true,
            show_weather: true,
            show_temp: true,
            show_humidity: true,
            show_wind: true,
            show_precip: true,
            show_pressure: false,
            show_visibility: false,
            show_uv: false,
            show_clouds: false,
            show_wind_dir: false,
            show_aqi: false,
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
    pub fn load_from_file() -> Result<Self> {
        let content = currents_core::config::load_config_file()?;
        let config: Config = toml::from_str(&content)
            .with_context(|| "Failed to parse forecast configuration")?;
        Ok(config)
    }
}

