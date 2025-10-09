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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastDay {
    pub date: chrono::DateTime<chrono::Utc>,
    pub temp_min: f64,
    pub temp_max: f64,
    pub humidity: f64,
    pub wind_speed: f64,
    pub description: String,
    pub precipitation_probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastData {
    pub location: String,
    pub days: Vec<ForecastDay>,
}

#[derive(Debug, Clone)]
pub struct WeatherFetcher {
    client: Client,
    api_key: String,
    location: String,
    units: String,
    provider: String,
    api_stats: Option<ApiStatsTracker>,
    api_daily_limit: u64,
}

impl WeatherFetcher {
    pub fn new(api_key: String, location: String, units: String, provider: String, api_daily_limit: u64) -> Self {
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
            api_daily_limit,
        }
    }
    
    pub async fn fetch_weather(&self) -> Result<WeatherData> {
        // Check rate limit before making API call
        if let Some(ref tracker) = self.api_stats {
            match tracker.can_make_call(self.api_daily_limit) {
                Ok(true) => {
                    // We're under the limit, proceed
                }
                Ok(false) => {
                    return Err(anyhow::anyhow!(
                        "API rate limit reached ({} calls per day). Will reset at midnight UTC.",
                        self.api_daily_limit
                    ));
                }
                Err(e) => {
                    // Log the error but proceed (fail open)
                    tracing::warn!("Failed to check rate limit: {}. Proceeding with API call.", e);
                }
            }
        }

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

    pub async fn fetch_forecast(&self) -> Result<ForecastData> {
        // Check rate limit before making API call
        if let Some(ref tracker) = self.api_stats {
            match tracker.can_make_call(self.api_daily_limit) {
                Ok(true) => {
                    // We're under the limit, proceed
                }
                Ok(false) => {
                    return Err(anyhow::anyhow!(
                        "API rate limit reached ({} calls per day). Will reset at midnight UTC.",
                        self.api_daily_limit
                    ));
                }
                Err(e) => {
                    // Log the error but proceed (fail open)
                    tracing::warn!("Failed to check rate limit: {}. Proceeding with API call.", e);
                }
            }
        }

        let result = match self.provider.as_str() {
            "openweathermap" => self.fetch_openweathermap_forecast().await,
            "weatherapi" => self.fetch_weatherapi_forecast().await,
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

    async fn fetch_openweathermap_forecast(&self) -> Result<ForecastData> {
        let url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?q={}&appid={}&units={}&cnt=40",
            self.location, self.api_key, self.units
        );
        
        info!("Fetching forecast from OpenWeatherMap");
        
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
        
        let forecast_response: OpenWeatherMapForecastResponse = response
            .json()
            .await
            .context("Failed to parse OpenWeatherMap forecast response")?;
        
        // Group forecasts by day and calculate daily min/max
        let mut daily_forecasts: std::collections::HashMap<String, Vec<&OpenWeatherMapForecastItem>> = std::collections::HashMap::new();
        
        for item in &forecast_response.list {
            let date_key = chrono::DateTime::from_timestamp(item.dt, 0)
                .unwrap_or(chrono::Utc::now())
                .format("%Y-%m-%d")
                .to_string();
            daily_forecasts.entry(date_key).or_default().push(item);
        }
        
        let mut days = Vec::new();
        let mut sorted_dates: Vec<String> = daily_forecasts.keys().cloned().collect();
        sorted_dates.sort();
        
        for date_str in sorted_dates.iter().take(5) {
            if let Some(items) = daily_forecasts.get(date_str) {
                let temps: Vec<f64> = items.iter().map(|i| i.main.temp).collect();
                let temp_min = temps.iter().cloned().fold(f64::INFINITY, f64::min);
                let temp_max = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                
                let avg_humidity = items.iter().map(|i| i.main.humidity).sum::<f64>() / items.len() as f64;
                let avg_wind_speed = items.iter().map(|i| i.wind.speed).sum::<f64>() / items.len() as f64;
                
                // Use the description from the middle of the day (around noon)
                let mid_item = items[items.len() / 2];
                let description = mid_item.weather[0].description.clone();
                
                // Calculate precipitation probability (OpenWeatherMap provides pop field)
                let avg_pop = items.iter().map(|i| i.pop.unwrap_or(0.0)).sum::<f64>() / items.len() as f64;
                
                let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                    .context("Failed to parse date")?
                    .and_hms_opt(12, 0, 0)
                    .unwrap()
                    .and_utc();
                
                days.push(ForecastDay {
                    date,
                    temp_min,
                    temp_max,
                    humidity: avg_humidity,
                    wind_speed: avg_wind_speed,
                    description,
                    precipitation_probability: avg_pop,
                });
            }
        }
        
        Ok(ForecastData {
            location: forecast_response.city.name,
            days,
        })
    }

    async fn fetch_weatherapi_forecast(&self) -> Result<ForecastData> {
        let url = format!(
            "https://api.weatherapi.com/v1/forecast.json?key={}&q={}&days=5&aqi=no&alerts=no",
            self.api_key, self.location
        );
        
        info!("Fetching forecast from WeatherAPI");
        
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
        
        let forecast_response: WeatherApiForecastResponse = response
            .json()
            .await
            .context("Failed to parse WeatherAPI forecast response")?;
        
        let days = forecast_response
            .forecast
            .forecastday
            .iter()
            .map(|day| {
                let date = chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Utc::now().date_naive())
                    .and_hms_opt(12, 0, 0)
                    .unwrap()
                    .and_utc();
                
                ForecastDay {
                    date,
                    temp_min: day.day.mintemp_c,
                    temp_max: day.day.maxtemp_c,
                    humidity: day.day.avghumidity,
                    wind_speed: day.day.maxwind_kph / 3.6, // Convert km/h to m/s
                    description: day.day.condition.text.clone(),
                    precipitation_probability: day.day.daily_chance_of_rain / 100.0,
                }
            })
            .collect();
        
        Ok(ForecastData {
            location: forecast_response.location.name,
            days,
        })
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

// OpenWeatherMap Forecast response structures
#[derive(Debug, Deserialize)]
struct OpenWeatherMapForecastResponse {
    list: Vec<OpenWeatherMapForecastItem>,
    city: CityData,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMapForecastItem {
    dt: i64,
    main: MainData,
    wind: WindData,
    weather: Vec<WeatherInfo>,
    pop: Option<f64>, // Probability of precipitation
}

#[derive(Debug, Deserialize)]
struct CityData {
    name: String,
}

// WeatherAPI Forecast response structures
#[derive(Debug, Deserialize)]
struct WeatherApiForecastResponse {
    location: LocationData,
    forecast: ForecastDataApi,
}

#[derive(Debug, Deserialize)]
struct LocationData {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ForecastDataApi {
    forecastday: Vec<ForecastDayApi>,
}

#[derive(Debug, Deserialize)]
struct ForecastDayApi {
    date: String,
    day: DayData,
}

#[derive(Debug, Deserialize)]
struct DayData {
    maxtemp_c: f64,
    mintemp_c: f64,
    avghumidity: f64,
    maxwind_kph: f64,
    daily_chance_of_rain: f64,
    condition: ConditionData,
}
