use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, error, warn};
use crate::config::Config;
use crate::weather::WeatherFetcher;
use crate::alerts::AlertEngine;
use crate::notifications::NotificationManager;
use std::fs;
use std::path::Path;

pub struct WeatherAlertDaemon {
    config: Config,
    weather_fetcher: WeatherFetcher,
    alert_engine: AlertEngine,
    notification_manager: NotificationManager,
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
        
        let alert_engine = AlertEngine::new(config.alerts.clone());
        let notification_manager = NotificationManager::new(config.notifications.clone());
        
        Ok(Self {
            config,
            weather_fetcher,
            alert_engine,
            notification_manager,
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
                    let triggered_alerts = self.alert_engine.check_alerts(&weather);
                    
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
                
                let triggered_alerts = self.alert_engine.check_alerts(&weather);
                
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
    
    async fn poll_weather(&self) -> Result<crate::weather::WeatherData> {
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

    fn write_cache_snapshot(&self, weather: &crate::weather::WeatherData) -> Result<()> {
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
