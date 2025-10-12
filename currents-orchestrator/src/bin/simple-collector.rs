//! Simple weather collector for testing orchestrator integration
//!
//! This is a standalone collector that fetches weather data for a single location
//! and stores it in the database. It's designed to be spawned by the orchestrator.

use currents_core::api::WeatherFetcher;
use currents_storage::{WeatherStorage, types::StorageConfig};
use currents_orchestrator::CollectorConfig;
use anyhow::{Result, Context};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, error};

// CollectorConfig is now imported from the unified_config module

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration from environment
    let config = CollectorConfig::from_env()
        .context("Failed to load collector configuration")?;
    
    // Validate configuration
    config.validate()
        .context("Invalid collector configuration")?;

    info!("Starting simple collector for location: {} ({})", 
          config.location_name, config.location_id);
    info!("Collection interval: {} seconds", config.collection_interval);

    // Initialize storage
    let storage_config = StorageConfig {
        database_path: config.storage_path.clone(),
        max_history_days: 365,
        enable_compression: true,
        compression_threshold: 30,
    };
    
    let storage = WeatherStorage::new(&config.storage_path, storage_config)
        .context("Failed to initialize weather storage")?;

    // Create collection interval
    let mut interval = interval(Duration::from_secs(config.collection_interval));

    info!("Collector initialized successfully");

    // Main collection loop
    loop {
        interval.tick().await;
        
        match collect_and_store(&config, &storage).await {
            Ok(()) => {
                info!("Successfully collected and stored weather data");
            }
            Err(e) => {
                error!("Failed to collect weather data: {}", e);
            }
        }
    }
}

async fn collect_and_store(config: &CollectorConfig, storage: &WeatherStorage) -> Result<()> {
    info!("Fetching weather data for: {}", config.location_name);
    
    // Create weather fetcher
    let fetcher = WeatherFetcher::new(
        config.api_key.clone(),
        config.location_name.clone(),
        config.units.clone(),
        config.provider.clone(),
        1000, // API daily limit
    );
    
    // Fetch current weather
    let weather_data = fetcher.fetch_weather().await
        .context("Failed to fetch weather data")?;

    info!("Received weather: {}°C, {}%, {} m/s - {}", 
          weather_data.temperature,
          weather_data.humidity,
          weather_data.wind_speed,
          weather_data.description);

    // Store in database with location ID
    storage.store_weather_data(&config.location_id, &weather_data).await
        .context("Failed to store weather data")?;

    Ok(())
}
