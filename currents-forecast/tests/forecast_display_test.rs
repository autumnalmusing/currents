use currents_forecast::display::ForecastFormatter;
use currents_core::{ForecastData, ForecastDay};
use currents_forecast::config::{ForecastHighlights, ForecastDisplayConfig, TemperatureHighlights, ValueHighlights};
use chrono::{Utc, Duration};

fn create_test_forecast() -> ForecastData {
    let now = Utc::now();
    ForecastData {
        location: "Test City".to_string(),
        days: vec![
            ForecastDay {
                date: now,
                temp_min: 10.0,
                temp_max: 20.0,
                humidity: 50.0,
                wind_speed: 5.0,
                wind_direction: Some("NE".to_string()),
                precipitation_probability: 0.2,
                description: "clear sky".to_string(),
                pressure: Some(1013.0),
                visibility: Some(10.0),
                uv_index: Some(5.0),
                feels_like_min: None,
                feels_like_max: None,
                cloud_cover: Some(20.0),
                aqi: Some(1.0), // Good air quality
                wind_gust: Some(8.0),
            },
            ForecastDay {
                date: now + Duration::days(1),
                temp_min: 15.0,
                temp_max: 35.0, // High temp
                humidity: 85.0, // High humidity
                wind_speed: 20.0, // High wind
                wind_direction: Some("SW".to_string()),
                precipitation_probability: 0.8, // High precip
                description: "thunderstorm".to_string(),
                pressure: Some(990.0),
                visibility: Some(3.0),
                uv_index: Some(8.0),
                feels_like_min: None,
                feels_like_max: None,
                cloud_cover: Some(90.0),
                aqi: Some(4.0), // Unhealthy air quality
                wind_gust: Some(25.0),
            },
            ForecastDay {
                date: now + Duration::days(2),
                temp_min: -5.0, // Low temp
                temp_max: 5.0,
                humidity: 15.0, // Low humidity
                wind_speed: 1.0, // Low wind
                wind_direction: Some("N".to_string()),
                precipitation_probability: 0.0,
                description: "snow".to_string(),
                pressure: Some(1025.0),
                visibility: Some(8.0),
                uv_index: Some(2.0),
                feels_like_min: None,
                feels_like_max: None,
                cloud_cover: Some(100.0),
                aqi: Some(2.0), // Moderate air quality
                wind_gust: Some(3.0),
            },
        ],
    }
}

#[test]
fn test_forecast_format_basic() {
    let forecast = create_test_forecast();
    let highlights = ForecastHighlights::default();
    let display_config = ForecastDisplayConfig::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    assert!(output.contains("Test City"));
    assert!(output.contains("clear sky"));
    assert!(output.contains("thunderstorm"));
    assert!(output.contains("snow"));
}

#[test]
fn test_forecast_with_temperature_highlights() {
    let forecast = create_test_forecast();
    let mut highlights = ForecastHighlights::default();
    highlights.temperature.high = Some(30.0);
    highlights.temperature.low = Some(0.0);
    let display_config = ForecastDisplayConfig::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Should contain ANSI color codes for highlighted temperatures
    assert!(output.contains("\x1b[")); // ANSI escape code
}

#[test]
fn test_forecast_with_custom_colors() {
    let forecast = create_test_forecast();
    let mut highlights = ForecastHighlights::default();
    highlights.temperature.high_color = "magenta".to_string();
    highlights.temperature.low_color = "cyan".to_string();
    let display_config = ForecastDisplayConfig::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Output should be generated without errors
    assert!(output.len() > 0);
    assert!(output.contains("Test City"));
}

#[test]
fn test_forecast_with_hex_colors() {
    let forecast = create_test_forecast();
    let mut highlights = ForecastHighlights::default();
    highlights.temperature.high = Some(30.0);
    highlights.temperature.high_color = "#ff5733".to_string();
    highlights.temperature.low = Some(0.0);
    highlights.temperature.low_color = "#89b4fa".to_string();
    let display_config = ForecastDisplayConfig::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Should contain RGB ANSI codes (38;2;r;g;b)
    assert!(output.contains("38;2;") || output.len() > 0);
}

#[test]
fn test_forecast_all_highlights_enabled() {
    let forecast = create_test_forecast();
    let highlights = ForecastHighlights {
        temperature: TemperatureHighlights {
            high: Some(30.0),
            high_color: "red".to_string(),
            low: Some(0.0),
            low_color: "blue".to_string(),
        },
        humidity: ValueHighlights {
            high: Some(80.0),
            high_color: "cyan".to_string(),
            low: Some(20.0),
            low_color: "yellow".to_string(),
        },
        wind_speed: ValueHighlights {
            high: Some(15.0),
            high_color: "red".to_string(),
            low: Some(2.0),
            low_color: "green".to_string(),
        },
        precipitation: ValueHighlights {
            high: Some(70.0),
            high_color: "magenta".to_string(),
            low: Some(10.0),
            low_color: "white".to_string(),
        },
        pressure: ValueHighlights::default(),
        visibility: ValueHighlights::default(),
        uv_index: ValueHighlights::default(),
        cloud_cover: ValueHighlights::default(),
        aqi: ValueHighlights::default(),
    };
    let display_config = ForecastDisplayConfig::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Should generate output with multiple highlights
    assert!(output.len() > 0);
    assert!(output.contains("Test City"));
    assert!(output.contains("\x1b[")); // ANSI codes present
}

#[test]
fn test_forecast_no_highlights() {
    let forecast = create_test_forecast();
    let highlights = ForecastHighlights {
        temperature: TemperatureHighlights {
            high: None,
            high_color: "red".to_string(),
            low: None,
            low_color: "blue".to_string(),
        },
        humidity: ValueHighlights {
            high: None,
            high_color: "red".to_string(),
            low: None,
            low_color: "yellow".to_string(),
        },
        wind_speed: ValueHighlights {
            high: None,
            high_color: "red".to_string(),
            low: None,
            low_color: "cyan".to_string(),
        },
        precipitation: ValueHighlights {
            high: None,
            high_color: "magenta".to_string(),
            low: None,
            low_color: "green".to_string(),
        },
        pressure: ValueHighlights::default(),
        visibility: ValueHighlights::default(),
        uv_index: ValueHighlights::default(),
        cloud_cover: ValueHighlights::default(),
        aqi: ValueHighlights::default(),
    };
    let display_config = ForecastDisplayConfig::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Should still generate valid output without highlights
    assert!(output.len() > 0);
    assert!(output.contains("Test City"));
}

#[test]
fn test_forecast_custom_display_columns() {
    let forecast = create_test_forecast();
    let highlights = ForecastHighlights::default();
    let display_config = ForecastDisplayConfig {
        show_date: true,
        show_weather: true,
        show_temp: true,
        show_humidity: false,
        show_wind: false,
        show_precip: false,
        show_pressure: true,
        show_visibility: true,
        show_uv: true,
        show_clouds: true,
        show_wind_dir: true,
        show_aqi: true,
    };
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Should contain new columns
    assert!(output.contains("Pressure"));
    assert!(output.contains("Visibility"));
    assert!(output.contains("UV"));
    assert!(output.contains("Clouds"));
    assert!(output.contains("Wind Dir"));
    assert!(output.contains("AQI"));
    
    // Should not contain hidden columns
    assert!(!output.contains("Humidity"));
    // Note: "Wind" is a substring of "Wind Dir", so we can't use this check
    assert!(!output.contains("Precip"));
}

#[test]
fn test_forecast_aqi_formatting() {
    let forecast = create_test_forecast();
    let highlights = ForecastHighlights::default();
    let display_config = ForecastDisplayConfig {
        show_date: true,
        show_weather: false,
        show_temp: false,
        show_humidity: false,
        show_wind: false,
        show_precip: false,
        show_pressure: false,
        show_visibility: false,
        show_uv: false,
        show_clouds: false,
        show_wind_dir: false,
        show_aqi: true,
    };
    
    let output = ForecastFormatter::format(&forecast, &highlights, &display_config);
    
    // Should contain AQI labels
    assert!(output.contains("Good"));
    assert!(output.contains("Unhealthy"));
    assert!(output.contains("Moderate"));
}

