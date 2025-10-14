use anyhow::Result;
use currents_core::types::{WeatherData, PrecipitationData};
use currents_history::{PatternAnalyzer, HistoryConfig};
use currents_storage::{WeatherStorage, StorageConfig};
use chrono::Utc;
use tempfile::TempDir;

#[tokio::test]
async fn test_weather_history_storage() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_weather.db");
    
    let storage_config = StorageConfig::default();
    let storage = WeatherStorage::new(&db_path, storage_config)?;
    
    // Create test weather data
    let weather_data = WeatherData {
        temperature: 22.5,
        humidity: 65.0,
        wind_speed: 3.2,
        description: "partly cloudy".to_string(),
        precipitation: Some(PrecipitationData {
            intensity: "light".to_string(),
            probability: 0.3,
        }),
        timestamp: Utc::now(),
    };
    
    // Store the data
    storage.store_weather_data("test-location", &weather_data).await?;
    
    // Retrieve the data
    let retrieved_data = storage.get_weather_history(1).await?;
    assert_eq!(retrieved_data.len(), 1);
    assert_eq!(retrieved_data[0].temperature, 22.5);
    assert_eq!(retrieved_data[0].humidity, 65.0);
    assert_eq!(retrieved_data[0].wind_speed, 3.2);
    assert_eq!(retrieved_data[0].description, "partly cloudy");
    
    Ok(())
}

#[tokio::test]
async fn test_pattern_analysis() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_pattern.db");
    
    let storage_config = StorageConfig::default();
    let storage = WeatherStorage::new(&db_path, storage_config)?;
    
    // Create multiple weather data points for trend analysis
    let base_time = Utc::now() - chrono::Duration::days(2); // Use 2 days to ensure all data is within range
    for i in 0..10 {
        let weather_data = WeatherData {
            temperature: 20.0 + (i as f64 * 0.5), // Increasing temperature trend
            humidity: 60.0 + (i as f64 * 2.0),    // Increasing humidity trend
            wind_speed: 5.0 - (i as f64 * 0.2),   // Decreasing wind trend
            description: "test weather".to_string(),
            precipitation: None,
            timestamp: base_time + chrono::Duration::hours(i as i64 * 4), // 4 hours apart
        };
        
        storage.store_weather_data("test-location", &weather_data).await?;
    }
    
    // Test trend analysis
    let analyzer = PatternAnalyzer::new(&storage);
    let temp_trends = analyzer.analyze_trends("temperature", 3).await?; // Use 3 days to be safe
    
    assert_eq!(temp_trends.metric, "temperature");
    assert_eq!(temp_trends.trend_direction, "increasing");
    assert!(temp_trends.daily_change > 0.0);
    assert!(temp_trends.data_points >= 9); // Allow for some flexibility
    
    Ok(())
}

#[tokio::test]
async fn test_database_stats() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_stats.db");
    
    let storage_config = StorageConfig::default();
    let storage = WeatherStorage::new(&db_path, storage_config)?;
    
    // Add some test data
    for i in 0..5 {
        let weather_data = WeatherData {
            temperature: 15.0 + i as f64,
            humidity: 50.0 + i as f64 * 5.0,
            wind_speed: 2.0 + i as f64 * 0.5,
            description: format!("test weather {}", i),
            precipitation: None,
            timestamp: Utc::now() - chrono::Duration::hours(i as i64),
        };
        
        storage.store_weather_data("test-location", &weather_data).await?;
    }
    
    // Get database statistics
    let stats = storage.get_stats().await?;
    
    assert_eq!(stats.total_records, 5);
    assert!(stats.avg_temperature > 15.0);
    assert!(stats.min_temperature <= 15.0);
    assert!(stats.max_temperature >= 19.0);
    
    Ok(())
}

#[test]
fn test_history_config_default() {
    let config = HistoryConfig::default();
    
    assert!(config.database_path.contains("currents"));
    assert_eq!(config.max_history_days, 365);
    assert!(config.auto_collect);
    assert_eq!(config.collection_interval, 3600);
    assert!(config.enable_compression);
    assert_eq!(config.compression_threshold, 30);
}

#[test]
fn test_history_config_merge() {
    let mut base_config = HistoryConfig::default();
    let update_config = HistoryConfig {
        database_path: "/custom/path.db".to_string(),
        max_history_days: 180,
        auto_collect: false,
        collection_interval: 7200,
        collection_strategy: "threshold".to_string(),
        threshold_settings: None,
        enable_compression: false,
        compression_threshold: 60,
    };
    
    base_config.merge_with(&update_config);
    
    assert_eq!(base_config.database_path, "/custom/path.db");
    assert_eq!(base_config.max_history_days, 180);
    assert!(!base_config.auto_collect);
    assert_eq!(base_config.collection_interval, 7200);
    assert!(!base_config.enable_compression);
    assert_eq!(base_config.compression_threshold, 60);
}