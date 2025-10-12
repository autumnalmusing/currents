use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, error, warn};
use crate::config::Config;
use currents_core::{WeatherFetcher, WeatherData};
use crate::alerts::AlertEngine;
use crate::notifications::NotificationManager;
use currents_history::CollectionManager;
use currents_storage::{WeatherStorage, StorageConfig};
use std::fs;
use std::path::Path;
use std::cell::RefCell;

pub struct WeatherAlertDaemon {
    config: Config,
    weather_fetcher: WeatherFetcher,
    alert_engine: RefCell<AlertEngine>,
    notification_manager: NotificationManager,
    storage: Option<WeatherStorage>,
    collection_manager: Option<RefCell<CollectionManager>>,
}

impl WeatherAlertDaemon {
    pub async fn new(config: Config) -> Result<Self> {
        let weather_fetcher = WeatherFetcher::new(
            config.weather.api_key.clone(),
            config.weather.location.clone(),
            config.weather.units.clone(),
            config.weather.provider.clone(),
            config.weather.api_daily_limit,
        );
        
        let alert_engine = RefCell::new(AlertEngine::new(config.alerts.clone()));
        let notification_manager = NotificationManager::new(config.notifications.clone());
        
        // Initialize storage if configured
        let (storage, collection_manager) = if let Some(history_config) = &config.history {
            if history_config.auto_collect {
                let storage_config = StorageConfig {
                    database_path: history_config.database_path.clone(),
                    max_history_days: history_config.max_history_days,
                    enable_compression: history_config.enable_compression,
                    compression_threshold: history_config.compression_threshold,
                };
                match WeatherStorage::new(&storage_config.database_path, storage_config.clone()) {
                    Ok(s) => {
                        info!("Weather storage enabled: {}", history_config.database_path);
                        info!("Collection strategy: {}", history_config.collection_strategy);
                        let collection_mgr = CollectionManager::new(history_config.clone());
                        (Some(s), Some(RefCell::new(collection_mgr)))
                    }
                    Err(e) => {
                        warn!("Failed to initialize weather storage: {}", e);
                        (None, None)
                    }
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        
        Ok(Self {
            config,
            weather_fetcher,
            alert_engine,
            notification_manager,
            storage,
            collection_manager,
        })
    }
    
    pub async fn run(&self) -> Result<()> {
        info!("Starting weather alert daemon");
        
        // Send a startup notification
        if let Err(e) = self.notification_manager.send_test_notification().await {
            warn!("Failed to send startup notification: {}", e);
        }
        
        loop {
            match self.poll_weather().await {
                Ok(weather) => {
                    // Cache snapshot for Waybar
                    if let Err(e) = self.write_cache_snapshot(&weather) {
                        warn!("Failed to write cache snapshot: {}", e);
                    }
                    
                    // Log weather data to storage if enabled (with smart collection)
                    if let (Some(ref storage), Some(ref collection_mgr)) = (&self.storage, &self.collection_manager) {
                        let mut manager = collection_mgr.borrow_mut();
                        if let Ok(should_collect) = manager.should_collect(&weather, storage).await {
                            if should_collect {
                                if let Err(e) = manager.collect(&weather, storage).await {
                                    warn!("Failed to store weather data in storage: {}", e);
                                } else {
                                    info!("Weather data collected using {} strategy", 
                                          self.config.history.as_ref().unwrap().collection_strategy);
                                }
                            }
                        }
                    }
                    
                    let triggered_alerts = self.alert_engine.borrow_mut().check_alerts(&weather);
                    
                    for alert in triggered_alerts {
                        if let Err(e) = self.notification_manager.send_alert(&alert).await {
                            error!("Failed to send alert {}: {}", alert.rule_name, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch weather data: {}", e);
                    // Continue running even if weather fetch fails
                }
            }
            
            // Wait for the configured interval
            sleep(Duration::from_secs(self.config.polling.interval_seconds)).await;
        }
    }
    
    pub async fn send_test_notification(&self) -> Result<()> {
        info!("Sending test notification");
        self.notification_manager.send_test_notification().await?;
        Ok(())
    }
    
    pub async fn send_custom_notification(&self, title: &str, message: &str) -> Result<()> {
        info!("Sending custom notification: {} - {}", title, message);
        self.notification_manager.send_custom_notification(title, message).await?;
        Ok(())
    }
    
    pub async fn run_foreground(&self) -> Result<()> {
        info!("Running in foreground mode");
        
        // Send a test notification
        if let Err(e) = self.notification_manager.send_test_notification().await {
            warn!("Failed to send test notification: {}", e);
        }
        
        // Run one iteration and exit
        match self.poll_weather().await {
            Ok(weather) => {
                info!("Weather data: {:.1}°C, {:.1}% humidity, {:.1} m/s wind", 
                      weather.temperature, weather.humidity, weather.wind_speed);
                info!("Description: {}", weather.description);
                
                // Log weather data to storage if enabled (with smart collection)
                if let (Some(ref storage), Some(ref collection_mgr)) = (&self.storage, &self.collection_manager) {
                    let mut manager = collection_mgr.borrow_mut();
                    if let Ok(should_collect) = manager.should_collect(&weather, storage).await {
                        if should_collect {
                            if let Err(e) = manager.collect(&weather, storage).await {
                                warn!("Failed to store weather data in storage: {}", e);
                            } else {
                                info!("Weather data collected using {} strategy", 
                                      self.config.history.as_ref().unwrap().collection_strategy);
                            }
                        } else {
                            info!("Weather data not collected (thresholds not met)");
                        }
                    }
                }
                
                let triggered_alerts = self.alert_engine.borrow_mut().check_alerts(&weather);
                
                if triggered_alerts.is_empty() {
                    info!("No alerts triggered");
                } else {
                    for alert in triggered_alerts {
                        info!("Alert would be triggered: {}", alert.rule_name);
                        if let Err(e) = self.notification_manager.send_alert(&alert).await {
                            error!("Failed to send alert {}: {}", alert.rule_name, e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to fetch weather data: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }
    
    async fn poll_weather(&self) -> Result<WeatherData> {
        let mut last_error = None;
        
        for attempt in 1..=self.config.polling.retry_attempts {
            match self.weather_fetcher.fetch_weather().await {
                Ok(weather) => {
                    if attempt > 1 {
                        info!("Weather fetch succeeded on attempt {}", attempt);
                    }
                    return Ok(weather);
                }
                Err(e) => {
                    warn!("Weather fetch attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    
                    if attempt < self.config.polling.retry_attempts {
                        sleep(Duration::from_secs(self.config.polling.retry_delay_seconds)).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All retry attempts failed")))
    }

    fn write_cache_snapshot(&self, weather: &WeatherData) -> Result<()> {
        #[derive(serde::Serialize)]
        struct WaybarSnapshot<'a> {
            text: String,
            tooltip: String,
            class: String,
            #[serde(rename = "alt")]
            alt_text: String,
            temperature: f64,
            humidity: f64,
            wind_speed: f64,
            description: &'a str,
            timestamp: i64,
        }

        let class = if weather.description.to_lowercase().contains("rain") {
            "rain"
        } else if weather.description.to_lowercase().contains("storm") {
            "storm"
        } else if weather.description.to_lowercase().contains("cloud") {
            "clouds"
        } else if weather.description.to_lowercase().contains("clear") {
            "clear"
        } else {
            "other"
        };

        let text = format!("{:.0}° | {}", weather.temperature, weather.description);
        let tooltip = format!(
            "{}\nTemp: {:.1}°\nHumidity: {:.0}%\nWind: {:.1} m/s",
            weather.description, weather.temperature, weather.humidity, weather.wind_speed
        );

        let snapshot = WaybarSnapshot {
            text,
            tooltip,
            class: class.to_string(),
            alt_text: weather.description.clone(),
            temperature: weather.temperature,
            humidity: weather.humidity,
            wind_speed: weather.wind_speed,
            description: &weather.description,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let json = serde_json::to_string(&snapshot)?;

        let cache_path = Path::new(&self.config.cache.path);
        if let Some(dir) = cache_path.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::write(cache_path, json)?;
        Ok(())
    }
}
