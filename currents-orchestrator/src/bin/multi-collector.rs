//! Multi-location weather collector for efficient data collection
//!
//! This collector handles multiple locations (5-10 per process) to optimize
//! resource usage for large-scale deployments.

use currents_core::api::WeatherFetcher;
use currents_storage::{WeatherStorage, types::StorageConfig};
use anyhow::{Result, Context};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use futures::future;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocationConfig {
    id: String,
    name: String,
    coordinates: (f64, f64),
    api_key: String,
    provider: String,
    units: String,
    collection_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MultiCollectorConfig {
    locations: Vec<LocationConfig>,
    storage_path: String,
    batch_size: usize,
    max_retries: u32,
    retry_delay: u64,
}

impl MultiCollectorConfig {
    fn from_env() -> Result<Self> {
        // Load configuration from environment variables
        let locations_json = std::env::var("COLLECTOR_LOCATIONS")
            .context("COLLECTOR_LOCATIONS environment variable not set")?;
        
        let locations: Vec<LocationConfig> = serde_json::from_str(&locations_json)
            .context("Failed to parse COLLECTOR_LOCATIONS JSON")?;
        
        let storage_path = std::env::var("COLLECTOR_STORAGE_PATH")
            .unwrap_or_else(|_| "~/.config/currents/orchestrator.db".to_string());
        
        let batch_size = std::env::var("COLLECTOR_BATCH_SIZE")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<usize>()
            .unwrap_or(5);
        
        let max_retries = std::env::var("COLLECTOR_MAX_RETRIES")
            .unwrap_or_else(|_| "3".to_string())
            .parse::<u32>()
            .unwrap_or(3);
        
        let retry_delay = std::env::var("COLLECTOR_RETRY_DELAY")
            .unwrap_or_else(|_| "60".to_string())
            .parse::<u64>()
            .unwrap_or(60);
        
        Ok(MultiCollectorConfig {
            locations,
            storage_path,
            batch_size,
            max_retries,
            retry_delay,
        })
    }
    
    fn validate(&self) -> Result<()> {
        if self.locations.is_empty() {
            return Err(anyhow::anyhow!("No locations configured"));
        }
        
        if self.locations.len() > 20 {
            return Err(anyhow::anyhow!("Too many locations per collector (max 20)"));
        }
        
        for location in &self.locations {
            if location.api_key.is_empty() {
                return Err(anyhow::anyhow!("Empty API key for location: {}", location.id));
            }
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration from environment
    let config = MultiCollectorConfig::from_env()
        .context("Failed to load collector configuration")?;
    
    // Validate configuration
    config.validate()
        .context("Invalid collector configuration")?;

    info!("Starting multi-location collector for {} locations", config.locations.len());
    info!("Batch size: {}, Max retries: {}", config.batch_size, config.max_retries);

    // Initialize storage
    let storage_config = StorageConfig {
        database_path: config.storage_path.clone(),
        max_history_days: 365,
        enable_compression: true,
        compression_threshold: 30,
    };
    
    let storage = WeatherStorage::new(&config.storage_path, storage_config)
        .context("Failed to initialize weather storage")?;

    // Create collection interval (use the minimum interval from all locations)
    let min_interval = config.locations.iter()
        .map(|loc| loc.collection_interval)
        .min()
        .unwrap_or(1800); // Default to 30 minutes
    
    let mut interval = interval(Duration::from_secs(min_interval));
    info!("Collection interval: {} seconds", min_interval);

    info!("Multi-location collector initialized successfully");

    // Main collection loop
    loop {
        interval.tick().await;
        
        match collect_all_locations(&config, &storage).await {
            Ok(()) => {
                info!("Successfully collected weather data for all locations");
            }
            Err(e) => {
                error!("Failed to collect weather data: {}", e);
            }
        }
    }
}

async fn collect_all_locations(config: &MultiCollectorConfig, storage: &WeatherStorage) -> Result<()> {
    // Group locations by API key to optimize API calls
    let mut api_groups: HashMap<String, Vec<&LocationConfig>> = HashMap::new();
    
    for location in &config.locations {
        api_groups.entry(location.api_key.clone())
            .or_insert_with(Vec::new)
            .push(location);
    }
    
    info!("Processing {} API groups", api_groups.len());
    
    // Process each API group
    for (api_key, locations) in api_groups {
        info!("Processing {} locations with API key: {}...", 
              locations.len(), 
              &api_key[..8]); // Show first 8 chars for security
        
        // Process locations in batches
        for batch in locations.chunks(config.batch_size) {
            let batch_futures: Vec<_> = batch.iter()
                .map(|location| collect_single_location(location, storage))
                .collect();
            
            // Wait for all locations in this batch to complete
            let results = future::join_all(batch_futures).await;
            
            // Log results
            let mut success_count = 0;
            let mut error_count = 0;
            
            for result in results {
                match result {
                    Ok(()) => success_count += 1,
                    Err(e) => {
                        error_count += 1;
                        error!("Location collection failed: {}", e);
                    }
                }
            }
            
            info!("Batch completed: {} success, {} errors", success_count, error_count);
            
            // Small delay between batches to avoid overwhelming the API
            if batch.len() == config.batch_size {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }
    
    Ok(())
}

async fn collect_single_location(location: &LocationConfig, storage: &WeatherStorage) -> Result<()> {
    // Create weather fetcher for this location
    let fetcher = WeatherFetcher::new(
        location.api_key.clone(),
        location.name.clone(),
        location.units.clone(),
        location.provider.clone(),
        1000, // API daily limit
    );
    
    // Fetch current weather with retries
    let mut attempts = 0;
    let weather_data = loop {
        match fetcher.fetch_weather().await {
            Ok(data) => break data,
            Err(e) => {
                attempts += 1;
                if attempts >= 3 {
                    return Err(e);
                }
                warn!("Attempt {} failed for {}: {}, retrying...", attempts, location.name, e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    };

    info!("Collected weather for {}: {}°C, {}%, {} m/s - {}", 
          location.name,
          weather_data.temperature,
          weather_data.humidity,
          weather_data.wind_speed,
          weather_data.description);

    // Store in database with location ID
    storage.store_weather_data(&location.id, &weather_data).await
        .context("Failed to store weather data")?;

    Ok(())
}
