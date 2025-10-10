use serde::{Deserialize, Serialize};

/// Current weather data snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub temperature: f64,
    pub humidity: f64,
    pub wind_speed: f64,
    pub description: String,
    pub precipitation: Option<PrecipitationData>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Precipitation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecipitationData {
    pub intensity: String,
    pub probability: f64,
}

/// Single day in a forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDay {
    pub date: chrono::DateTime<chrono::Utc>,
    pub temp_min: f64,
    pub temp_max: f64,
    pub humidity: f64,
    pub wind_speed: f64,
    pub wind_direction: Option<String>, // Wind direction (e.g., "N", "NE", "E", etc.)
    pub description: String,
    pub precipitation_probability: f64,
    pub pressure: Option<f64>,        // Atmospheric pressure in hPa/mb
    pub visibility: Option<f64>,      // Visibility in km
    pub uv_index: Option<f64>,        // UV index
    pub feels_like_min: Option<f64>,  // Feels like temperature min
    pub feels_like_max: Option<f64>,  // Feels like temperature max
    pub cloud_cover: Option<f64>,     // Cloud cover percentage
    pub aqi: Option<f64>,              // Air Quality Index (US EPA standard)
    pub wind_gust: Option<f64>,        // Maximum wind gust speed in m/s
}

/// Multi-day forecast data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastData {
    pub location: String,
    pub days: Vec<ForecastDay>,
}

