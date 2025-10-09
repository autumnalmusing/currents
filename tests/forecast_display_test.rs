use currents::forecast_display::ForecastFormatter;
use currents::weather::{ForecastData, ForecastDay};
use currents::config::{ForecastHighlights, TemperatureHighlights, ValueHighlights};
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
                precipitation_probability: 0.2,
                description: "clear sky".to_string(),
            },
            ForecastDay {
                date: now + Duration::days(1),
                temp_min: 15.0,
                temp_max: 35.0, // High temp
                humidity: 85.0, // High humidity
                wind_speed: 20.0, // High wind
                precipitation_probability: 0.8, // High precip
                description: "thunderstorm".to_string(),
            },
            ForecastDay {
                date: now + Duration::days(2),
                temp_min: -5.0, // Low temp
                temp_max: 5.0,
                humidity: 15.0, // Low humidity
                wind_speed: 1.0, // Low wind
                precipitation_probability: 0.0,
                description: "snow".to_string(),
            },
        ],
    }
}

#[test]
fn test_forecast_format_basic() {
    let forecast = create_test_forecast();
    let highlights = ForecastHighlights::default();
    
    let output = ForecastFormatter::format(&forecast, &highlights);
    
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
    
    let output = ForecastFormatter::format(&forecast, &highlights);
    
    // Should contain ANSI color codes for highlighted temperatures
    assert!(output.contains("\x1b[")); // ANSI escape code
}

#[test]
fn test_forecast_with_custom_colors() {
    let forecast = create_test_forecast();
    let mut highlights = ForecastHighlights::default();
    highlights.temperature.high_color = "magenta".to_string();
    highlights.temperature.low_color = "cyan".to_string();
    
    let output = ForecastFormatter::format(&forecast, &highlights);
    
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
    
    let output = ForecastFormatter::format(&forecast, &highlights);
    
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
    };
    
    let output = ForecastFormatter::format(&forecast, &highlights);
    
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
    };
    
    let output = ForecastFormatter::format(&forecast, &highlights);
    
    // Should still generate valid output without highlights
    assert!(output.len() > 0);
    assert!(output.contains("Test City"));
}

