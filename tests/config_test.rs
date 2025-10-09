use currents::config::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_load_toml_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    let config_content = r#"
[weather]
api_key = "test-key-123"
location = "Test City,TC"
units = "metric"
provider = "openweathermap"
api_daily_limit = 500

[notifications]
urgency = "normal"
timeout = 5000
sound = true

[polling]
interval_seconds = 300
retry_attempts = 3
retry_delay_seconds = 60

[[alerts]]
name = "test-alert"
enabled = true
message = "Test message"
repeat = "once"

[alerts.condition.temperature]
min = 30.0
"#;
    
    fs::write(&config_path, config_content).unwrap();
    
    let config = Config::load(&config_path).unwrap();
    assert_eq!(config.weather.api_key, "test-key-123");
    assert_eq!(config.weather.location, "Test City,TC");
    assert_eq!(config.weather.units, "metric");
    assert_eq!(config.weather.provider, "openweathermap");
    assert_eq!(config.weather.api_daily_limit, 500);
    assert_eq!(config.alerts.len(), 1);
    assert_eq!(config.alerts[0].name, "test-alert");
    assert_eq!(config.alerts[0].repeat, "once");
}

#[test]
fn test_default_cache_config() {
    let cache = CacheConfig::default();
    assert!(cache.path.contains(".cache/currents/weather.json"));
    assert_eq!(cache.ttl_seconds, 300);
}

#[test]
fn test_default_forecast_highlights() {
    let highlights = ForecastHighlights::default();
    assert_eq!(highlights.temperature.high, Some(30.0));
    assert_eq!(highlights.temperature.low, Some(0.0));
    assert_eq!(highlights.temperature.high_color, "red");
    assert_eq!(highlights.temperature.low_color, "blue");
    assert_eq!(highlights.humidity.high, None);
    assert_eq!(highlights.wind_speed.high, None);
}

#[test]
fn test_alert_repeat_default() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    let config_content = r#"
[weather]
api_key = "test"
location = "Test"
units = "metric"
provider = "openweathermap"

[notifications]
urgency = "normal"
timeout = 5000
sound = true

[polling]
interval_seconds = 300
retry_attempts = 3
retry_delay_seconds = 60

[[alerts]]
name = "no-repeat-specified"
enabled = true
message = "Test"

[alerts.condition.temperature]
min = 30.0
"#;
    
    fs::write(&config_path, config_content).unwrap();
    let config = Config::load(&config_path).unwrap();
    
    // Should default to "once"
    assert_eq!(config.alerts[0].repeat, "once");
}

#[test]
fn test_custom_forecast_colors() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    
    let config_content = r##"
[weather]
api_key = "test"
location = "Test"
units = "metric"
provider = "openweathermap"

[notifications]
urgency = "normal"
timeout = 5000
sound = true

[polling]
interval_seconds = 300
retry_attempts = 3
retry_delay_seconds = 60

[forecast_highlights.temperature]
high = 35.0
high_color = "#ff5733"
low = -5.0
low_color = "cyan"

[forecast_highlights.humidity]
high = 80.0
high_color = "blue"

[[alerts]]
name = "test"
enabled = true
message = "test"
[alerts.condition]
"##;
    
    fs::write(&config_path, config_content).unwrap();
    let config = Config::load(&config_path).unwrap();
    
    assert_eq!(config.forecast_highlights.temperature.high, Some(35.0));
    assert_eq!(config.forecast_highlights.temperature.high_color, "#ff5733");
    assert_eq!(config.forecast_highlights.temperature.low, Some(-5.0));
    assert_eq!(config.forecast_highlights.temperature.low_color, "cyan");
    assert_eq!(config.forecast_highlights.humidity.high, Some(80.0));
    assert_eq!(config.forecast_highlights.humidity.high_color, "blue");
}

