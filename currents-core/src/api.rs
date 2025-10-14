use anyhow::{Result, Context};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;
use crate::api_stats::ApiStatsTracker;
use crate::types::{WeatherData, PrecipitationData, ForecastDay, ForecastData};

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
    
    /// Convert degrees to compass direction (N, NE, E, SE, S, SW, W, NW)
    fn degrees_to_compass(degrees: f64) -> String {
        let directions = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
        let index = ((degrees + 22.5) / 45.0) as usize % 8;
        directions[index].to_string()
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

    /// Fetch historical weather data for a specific date
    pub async fn fetch_historical_weather(&self, date: chrono::DateTime<chrono::Utc>) -> Result<WeatherData> {
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
            "openweathermap" => self.fetch_openweathermap_historical(date).await,
            "weatherapi" => self.fetch_weatherapi_historical(date).await,
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

    /// Fetch historical weather data for a date range (bulk operation)
    pub async fn fetch_historical_range(
        &self, 
        start_date: chrono::DateTime<chrono::Utc>, 
        end_date: chrono::DateTime<chrono::Utc>
    ) -> Result<Vec<WeatherData>> {
        let mut results = Vec::new();
        let mut current_date = start_date;
        
        // Add a small delay between requests to respect rate limits
        let delay = std::time::Duration::from_millis(100);
        
        while current_date <= end_date {
            match self.fetch_historical_weather(current_date).await {
                Ok(weather_data) => {
                    results.push(weather_data);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch historical data for {}: {}", current_date.format("%Y-%m-%d"), e);
                    // Continue with next date instead of failing completely
                }
            }
            
            current_date = current_date + chrono::Duration::days(1);
            
            // Add delay between requests to avoid overwhelming the API
            tokio::time::sleep(delay).await;
        }
        
        Ok(results)
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
                
                // Calculate average pressure and cloud cover
                let avg_pressure = items.iter().map(|i| i.main.pressure.unwrap_or(0.0)).sum::<f64>() / items.len() as f64;
                let avg_cloud_cover = items.iter().map(|i| i.clouds.all).sum::<f64>() / items.len() as f64;
                
                // Visibility: OpenWeatherMap doesn't provide visibility in 5-day forecast, only in current weather
                // UV: Also not available in basic forecast endpoint
                
                // Calculate average wind direction (in degrees)
                let avg_wind_deg = items.iter().map(|i| i.wind.deg.unwrap_or(0.0)).sum::<f64>() / items.len() as f64;
                let wind_direction = Self::degrees_to_compass(avg_wind_deg);
                
                // Calculate maximum wind gust
                let max_gust = items.iter()
                    .filter_map(|i| i.wind.gust)
                    .fold(0.0f64, f64::max);
                
                days.push(ForecastDay {
                    date,
                    temp_min,
                    temp_max,
                    humidity: avg_humidity,
                    wind_speed: avg_wind_speed,
                    wind_direction: Some(wind_direction),
                    description,
                    precipitation_probability: avg_pop,
                    pressure: if avg_pressure > 0.0 { Some(avg_pressure) } else { None },
                    visibility: None, // Not available in OpenWeatherMap forecast
                    uv_index: None,   // Not available in OpenWeatherMap forecast
                    feels_like_min: None, // Could calculate but not provided directly
                    feels_like_max: None,
                    cloud_cover: Some(avg_cloud_cover),
                    aqi: None, // Not available in OpenWeatherMap forecast
                    wind_gust: if max_gust > 0.0 { Some(max_gust) } else { None },
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
            "https://api.weatherapi.com/v1/forecast.json?key={}&q={}&days=5&aqi=yes&alerts=no",
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
                    wind_direction: day.day.wind_dir.clone(),
                    description: day.day.condition.text.clone(),
                    precipitation_probability: day.day.daily_chance_of_rain / 100.0,
                    pressure: day.day.avgpressure_mb,
                    visibility: day.day.avgvis_km,
                    uv_index: day.day.uv,
                    feels_like_min: None, // WeatherAPI provides hourly feels_like but not daily min/max
                    feels_like_max: None,
                    cloud_cover: day.day.cloud_cover,
                    aqi: day.day.air_quality.as_ref().and_then(|aq| aq.us_epa_index),
                    wind_gust: day.day.maxwind_gust_kph.map(|g| g / 3.6), // Convert km/h to m/s
                }
            })
            .collect();
        
        Ok(ForecastData {
            location: forecast_response.location.name,
            days,
        })
    }

    async fn fetch_openweathermap_historical(&self, date: chrono::DateTime<chrono::Utc>) -> Result<WeatherData> {
        let timestamp = date.timestamp();
        let url = format!(
            "https://api.openweathermap.org/data/2.5/onecall/timemachine?lat={}&lon={}&dt={}&appid={}&units={}",
            // Note: OpenWeatherMap historical API requires lat/lon, not city name
            // For now, we'll use a default location (London) - this should be configurable
            "51.5074", "0.1278", timestamp, self.api_key, self.units
        );
        
        info!("Fetching historical weather from OpenWeatherMap for {}", date.format("%Y-%m-%d"));
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to OpenWeatherMap historical API")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "OpenWeatherMap historical API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        
        let historical_response: OpenWeatherMapHistoricalResponse = response
            .json()
            .await
            .context("Failed to parse OpenWeatherMap historical response")?;
        
        Ok(WeatherData {
            temperature: historical_response.current.temp,
            humidity: historical_response.current.humidity,
            wind_speed: historical_response.current.wind_speed,
            description: historical_response.current.weather[0].description.clone(),
            precipitation: Some(PrecipitationData {
                intensity: "none".to_string(), // Historical data doesn't include precipitation intensity
                probability: 0.0,
            }),
            timestamp: date,
        })
    }

    async fn fetch_weatherapi_historical(&self, date: chrono::DateTime<chrono::Utc>) -> Result<WeatherData> {
        let date_str = date.format("%Y-%m-%d").to_string();
        let url = format!(
            "https://api.weatherapi.com/v1/history.json?key={}&q={}&dt={}",
            self.api_key, self.location, date_str
        );
        
        info!("Fetching historical weather from WeatherAPI for {}", date_str);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .context("Failed to send request to WeatherAPI historical API")?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "WeatherAPI historical error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }
        
        let historical_response: WeatherApiHistoricalResponse = response
            .json()
            .await
            .context("Failed to parse WeatherAPI historical response")?;
        
        Ok(WeatherData {
            temperature: historical_response.forecast.forecastday[0].day.avgtemp_c,
            humidity: historical_response.forecast.forecastday[0].day.avghumidity,
            wind_speed: historical_response.forecast.forecastday[0].day.maxwind_kph / 3.6, // Convert km/h to m/s
            description: historical_response.forecast.forecastday[0].day.condition.text.clone(),
            precipitation: Some(PrecipitationData {
                intensity: self.classify_precipitation_intensity(historical_response.forecast.forecastday[0].day.totalprecip_mm),
                probability: historical_response.forecast.forecastday[0].day.daily_chance_of_rain / 100.0,
            }),
            timestamp: date,
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
    pressure: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct WindData {
    speed: f64,
    deg: Option<f64>, // Wind direction in degrees
    gust: Option<f64>, // Wind gust speed
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
    clouds: CloudsData,
}

#[derive(Debug, Deserialize)]
struct CloudsData {
    all: f64, // Cloud cover percentage
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
    avgpressure_mb: Option<f64>,
    avgvis_km: Option<f64>,
    uv: Option<f64>,
    cloud_cover: Option<f64>,
    wind_dir: Option<String>, // Wind direction (e.g., "N", "NE", etc.)
    maxwind_gust_kph: Option<f64>, // Maximum wind gust
    air_quality: Option<AirQuality>,
}

#[derive(Debug, Deserialize)]
struct AirQuality {
    #[serde(rename = "us-epa-index")]
    us_epa_index: Option<f64>, // US EPA Air Quality Index (1-6 scale)
}

// OpenWeatherMap Historical API response structures
#[derive(Debug, Deserialize)]
struct OpenWeatherMapHistoricalResponse {
    current: OpenWeatherMapHistoricalCurrent,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherMapHistoricalCurrent {
    temp: f64,
    humidity: f64,
    wind_speed: f64,
    weather: Vec<WeatherInfo>,
}

// WeatherAPI Historical API response structures
#[derive(Debug, Deserialize)]
struct WeatherApiHistoricalResponse {
    forecast: WeatherApiHistoricalForecast,
}

#[derive(Debug, Deserialize)]
struct WeatherApiHistoricalForecast {
    forecastday: Vec<WeatherApiHistoricalDay>,
}

#[derive(Debug, Deserialize)]
struct WeatherApiHistoricalDay {
    day: WeatherApiHistoricalDayData,
}

#[derive(Debug, Deserialize)]
struct WeatherApiHistoricalDayData {
    avgtemp_c: f64,
    avghumidity: f64,
    maxwind_kph: f64,
    condition: ConditionData,
    totalprecip_mm: f64,
    daily_chance_of_rain: f64,
}
