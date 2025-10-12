use anyhow::Result;
use chrono::{DateTime, Utc};
use currents_core::types::WeatherData;
use crate::config::{HistoryConfig, ThresholdSettings};
use currents_storage::WeatherStorage;

/// Smart collection manager that handles different collection strategies
pub struct CollectionManager {
    config: HistoryConfig,
    last_collected: Option<WeatherData>,
    last_collection_time: Option<DateTime<Utc>>,
}

impl CollectionManager {
    pub fn new(config: HistoryConfig) -> Self {
        Self {
            config,
            last_collected: None,
            last_collection_time: None,
        }
    }
    
    /// Determine if weather data should be collected based on strategy
    pub async fn should_collect(&mut self, weather: &WeatherData, storage: &WeatherStorage) -> Result<bool> {
        match self.config.collection_strategy.as_str() {
            "interval" => self.should_collect_interval(weather).await,
            "threshold" => self.should_collect_threshold(weather).await,
            "adaptive" => self.should_collect_adaptive(weather, storage).await,
            "hybrid" => self.should_collect_hybrid(weather, storage).await,
            _ => {
                // Default to interval strategy for unknown strategies
                self.should_collect_interval(weather).await
            }
        }
    }
    
    /// Collect weather data and update internal state
    pub async fn collect(&mut self, weather: &WeatherData, storage: &WeatherStorage) -> Result<()> {
        // Use "default" as location_id for single-location history
        storage.store_weather_data("default", weather).await?;
        self.last_collected = Some(weather.clone());
        self.last_collection_time = Some(Utc::now());
        Ok(())
    }
    
    /// Interval-based collection (current behavior)
    async fn should_collect_interval(&self, _weather: &WeatherData) -> Result<bool> {
        if let Some(last_time) = self.last_collection_time {
            let elapsed = Utc::now().signed_duration_since(last_time).num_seconds() as u64;
            Ok(elapsed >= self.config.collection_interval)
        } else {
            // First collection
            Ok(true)
        }
    }
    
    /// Threshold-based collection
    async fn should_collect_threshold(&self, weather: &WeatherData) -> Result<bool> {
        let Some(thresholds) = &self.config.threshold_settings else {
            // Fall back to interval if no thresholds configured
            return self.should_collect_interval(weather).await;
        };
        
        // Check minimum interval
        if let Some(last_time) = self.last_collection_time {
            let elapsed = Utc::now().signed_duration_since(last_time).num_seconds() as u64;
            if elapsed < thresholds.min_interval {
                return Ok(false);
            }
        }
        
        // Check maximum interval (force collection)
        if let Some(last_time) = self.last_collection_time {
            let elapsed = Utc::now().signed_duration_since(last_time).num_seconds() as u64;
            if elapsed >= thresholds.max_interval {
                return Ok(true);
            }
        }
        
        // Check if this is the first collection
        let Some(last_weather) = &self.last_collected else {
            return Ok(true);
        };
        
        // Check threshold crossings
        let temp_change = (weather.temperature - last_weather.temperature).abs();
        let humidity_change = (weather.humidity - last_weather.humidity).abs();
        let wind_change = (weather.wind_speed - last_weather.wind_speed).abs();
        
        let temp_threshold_crossed = temp_change >= thresholds.temperature_threshold;
        let humidity_threshold_crossed = humidity_change >= thresholds.humidity_threshold;
        let wind_threshold_crossed = wind_change >= thresholds.wind_speed_threshold;
        let description_changed = thresholds.track_description_changes && 
            weather.description != last_weather.description;
        
        // Collect if any threshold is crossed
        Ok(temp_threshold_crossed || humidity_threshold_crossed || 
           wind_threshold_crossed || description_changed)
    }
    
    /// Adaptive collection (adjusts intervals based on weather stability)
    async fn should_collect_adaptive(&mut self, weather: &WeatherData, storage: &WeatherStorage) -> Result<bool> {
        // Get recent weather data to analyze stability
        let recent_data = storage.get_weather_history(1).await?; // Last day
        
        if recent_data.len() < 10 {
            // Not enough data for adaptive analysis, use interval
            return self.should_collect_interval(weather).await;
        }
        
        // Calculate weather stability (lower volatility = more stable)
        let temps: Vec<f64> = recent_data.iter().map(|w| w.temperature).collect();
        let temp_volatility = self.calculate_volatility(&temps);
        
        // Adjust collection interval based on stability
        let base_interval = self.config.collection_interval;
        let adaptive_interval = if temp_volatility < 1.0 {
            // Very stable weather - collect less frequently
            base_interval * 2
        } else if temp_volatility > 3.0 {
            // Unstable weather - collect more frequently
            base_interval / 2
        } else {
            base_interval
        };
        
        // Check if enough time has passed with adaptive interval
        if let Some(last_time) = self.last_collection_time {
            let elapsed = Utc::now().signed_duration_since(last_time).num_seconds() as u64;
            Ok(elapsed >= adaptive_interval)
        } else {
            Ok(true)
        }
    }
    
    /// Hybrid collection (combines interval and threshold strategies)
    async fn should_collect_hybrid(&mut self, weather: &WeatherData, _storage: &WeatherStorage) -> Result<bool> {
        // Check interval-based collection
        let interval_should_collect = self.should_collect_interval(weather).await?;
        
        // Check threshold-based collection
        let threshold_should_collect = self.should_collect_threshold(weather).await?;
        
        // Collect if either condition is met
        Ok(interval_should_collect || threshold_should_collect)
    }
    
    /// Calculate volatility (standard deviation) of a data series
    fn calculate_volatility(&self, data: &[f64]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }
        
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / data.len() as f64;
        
        variance.sqrt()
    }
}

impl Default for ThresholdSettings {
    fn default() -> Self {
        Self {
            temperature_threshold: 2.0,      // 2°C change
            humidity_threshold: 10.0,        // 10% change
            wind_speed_threshold: 2.0,       // 2 m/s change
            pressure_threshold: 5.0,         // 5 hPa change
            track_description_changes: true,
            min_interval: 300,               // 5 minutes minimum
            max_interval: 7200,              // 2 hours maximum
            force_interval: Some(3600),      // Force collection every hour
        }
    }
}