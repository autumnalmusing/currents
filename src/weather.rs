use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;
use crate::api_stats::ApiStatsTracker;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub temperature: f64,
    pub humidity: f64,
    pub wind_speed: f64,
    pub description: String,
    pub precipitation: Option<PrecipitationData>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecipitationData {
    pub intensity: String,
    pub probability: f64,
}

#[derive(Debug, Clone)]
pub struct WeatherFetcher {
    client: Client,
    api_key: String,
    location: String,
    units: String,
    provider: String,
    api_stats: Option<ApiStatsTracker>,
}

impl WeatherFetcher {
    pub fn new(api_key: String, location: String, units: String, provider: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        
        // Initialize API stats tracker (ignore errors if it fails)
        let api_stats = ApiStatsTracker::with_default_path().ok();
        
        Self {
            client,
            api_key,
            location,
            units,
            provider,
            api_stats,
        }
    }
    
    pub async fn fetch_weather(&self) -> Result<WeatherData> {
        let result = match self.provider.as_str() {
            "openweathermap" => self.fetch_openweathermap().await,
            "weatherapi" => self.fetch_weatherapi().await,
            _ => Err(anyhow::anyhow!("Unsupported weather provider: {}", self.provider)),
        };
        
        // Increment API call counter if fetch was successful
        if result.is_ok() {
            if let Some(ref tracker) = self.api_stats {
                let _ = tracker.increment();
            }
        }
        
        result
    }
    
    async fn fetch_openweathermap(&self) -> Result<WeatherData> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units={}",
            self.location, self.api_key, self.units
        );
        
        info!("Fetching weather from OpenWeatherMap");
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to OpenWeatherMap")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "OpenWeatherMap API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        
        let weather_response: OpenWeatherMapResponse = response
            .json()
            .await
            .context("Failed to parse OpenWeatherMap response")?;
        
        Ok(WeatherData {
            temperature: weather_response.main.temp,
            humidity: weather_response.main.humidity,
            wind_speed: weather_response.wind.speed,
            description: weather_response.weather[0].description.clone(),
            precipitation: self.extract_precipitation(&weather_response),
            timestamp: chrono::Utc::now(),
        })
    }
    
    async fn fetch_weatherapi(&self) -> Result<WeatherData> {
        let url = format!(
            "https://api.weatherapi.com/v1/current.json?key={}&q={}&aqi=no",
            self.api_key, self.location
        );
        
        info!("Fetching weather from WeatherAPI");
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to WeatherAPI")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "WeatherAPI error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        
        let weather_response: WeatherApiResponse = response
            .json()
            .await
            .context("Failed to parse WeatherAPI response")?;
        
        Ok(WeatherData {
            temperature: weather_response.current.temp_c,
            humidity: weather_response.current.humidity,
            wind_speed: weather_response.current.wind_kph / 3.6, // Convert km/h to m/s
            description: weather_response.current.condition.text.clone(),
            precipitation: Some(PrecipitationData {
                intensity: self.classify_precipitation_intensity(weather_response.current.precip_mm),
                probability: 0.0, // WeatherAPI doesn't provide probability in current endpoint
            }),
            timestamp: chrono::Utc::now(),
        })
    }
    
    fn extract_precipitation(&self, _response: &OpenWeatherMapResponse) -> Option<PrecipitationData> {
        // OpenWeatherMap doesn't always provide precipitation data in current weather
        // This is a simplified implementation
        Some(PrecipitationData {
            intensity: "none".to_string(),
            probability: 0.0,
        })
    }
    
    fn classify_precipitation_intensity(&self, precip_mm: f64) -> String {
        if precip_mm == 0.0 {
            "none".to_string()
        } else if precip_mm < 2.5 {
            "light".to_string()
        } else if precip_mm < 10.0 {
            "moderate".to_string()
        } else {
            "heavy".to_string()
        }
    }
}

// OpenWeatherMap API response structures
#[derive(Debug, Deserialize)]
struct OpenWeatherMapResponse {
    main: MainData,
    wind: WindData,
    weather: Vec<WeatherInfo>,
}

#[derive(Debug, Deserialize)]
struct MainData {
    temp: f64,
    humidity: f64,
}

#[derive(Debug, Deserialize)]
struct WindData {
    speed: f64,
}

#[derive(Debug, Deserialize)]
struct WeatherInfo {
    description: String,
}

// WeatherAPI response structures
#[derive(Debug, Deserialize)]
struct WeatherApiResponse {
    current: CurrentData,
}

#[derive(Debug, Deserialize)]
struct CurrentData {
    temp_c: f64,
    humidity: f64,
    wind_kph: f64,
    condition: ConditionData,
    precip_mm: f64,
}

#[derive(Debug, Deserialize)]
struct ConditionData {
    text: String,
}
