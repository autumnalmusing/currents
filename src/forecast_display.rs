use crate::weather::ForecastData;
use crate::config::ForecastHighlights;
use tabled::{Table, Tabled, settings::{Style, Alignment, Modify, object::Columns, Color}};

pub struct ForecastFormatter;

#[derive(Tabled)]
struct ForecastColumn {
    #[tabled(rename = "Date")]
    date: String,
    #[tabled(rename = "Weather")]
    weather: String,
    #[tabled(rename = "Temp")]
    temp: String,
    #[tabled(rename = "Humidity")]
    humidity: String,
    #[tabled(rename = "Wind")]
    wind: String,
    #[tabled(rename = "Precip")]
    precip: String,
}

impl ForecastFormatter {
    pub fn format(forecast: &ForecastData, highlights: &ForecastHighlights) -> String {
        // Create a vector of ForecastColumn structs for tabled
        let columns: Vec<ForecastColumn> = forecast.days.iter()
            .map(|day| ForecastColumn {
                date: day.date.format("%a %m/%d").to_string(),
                weather: day.description.clone(),
                temp: format!("{}°-{}°", Self::format_temp(day.temp_min), Self::format_temp(day.temp_max)),
                humidity: format!("{}%", day.humidity as i32),
                wind: format!("{:.1}m/s", day.wind_speed),
                precip: format!("{}%", (day.precipitation_probability * 100.0) as i32),
            })
            .collect();
        
        let mut table = Table::new(&columns);
        table.with(Style::rounded())
             .with(Modify::new(Columns::new(..)).with(Alignment::center()));
        
        // Apply color highlights after table is built
        for (row_idx, day) in forecast.days.iter().enumerate() {
            let table_row = row_idx + 1; // +1 because row 0 is the header
            
            // Temperature highlighting (column 2)
            if let Some(high) = highlights.temperature.high {
                if day.temp_max >= high {
                    if let Some(color) = Self::parse_color(&highlights.temperature.high_color) {
                        table.with(Modify::new((table_row, 2)).with(color));
                    }
                }
            }
            if let Some(low) = highlights.temperature.low {
                if day.temp_min <= low {
                    if let Some(color) = Self::parse_color(&highlights.temperature.low_color) {
                        table.with(Modify::new((table_row, 2)).with(color));
                    }
                }
            }
            
            // Humidity highlighting (column 3)
            if let Some(high) = highlights.humidity.high {
                if day.humidity as f64 >= high {
                    if let Some(color) = Self::parse_color(&highlights.humidity.high_color) {
                        table.with(Modify::new((table_row, 3)).with(color));
                    }
                }
            }
            if let Some(low) = highlights.humidity.low {
                if (day.humidity as f64) <= low {
                    if let Some(color) = Self::parse_color(&highlights.humidity.low_color) {
                        table.with(Modify::new((table_row, 3)).with(color));
                    }
                }
            }
            
            // Wind speed highlighting (column 4)
            if let Some(high) = highlights.wind_speed.high {
                if day.wind_speed >= high {
                    if let Some(color) = Self::parse_color(&highlights.wind_speed.high_color) {
                        table.with(Modify::new((table_row, 4)).with(color));
                    }
                }
            }
            if let Some(low) = highlights.wind_speed.low {
                if day.wind_speed <= low {
                    if let Some(color) = Self::parse_color(&highlights.wind_speed.low_color) {
                        table.with(Modify::new((table_row, 4)).with(color));
                    }
                }
            }
            
            // Precipitation highlighting (column 5)
            let precip_percent = day.precipitation_probability * 100.0;
            if let Some(high) = highlights.precipitation.high {
                if precip_percent >= high {
                    if let Some(color) = Self::parse_color(&highlights.precipitation.high_color) {
                        table.with(Modify::new((table_row, 5)).with(color));
                    }
                }
            }
            if let Some(low) = highlights.precipitation.low {
                if precip_percent <= low {
                    if let Some(color) = Self::parse_color(&highlights.precipitation.low_color) {
                        table.with(Modify::new((table_row, 5)).with(color));
                    }
                }
            }
        }
        
        format!("\n5-Day Forecast for {}\n{}\n", forecast.location, table)
    }
    
    fn format_temp(temp: f64) -> String {
        format!("{:.0}", temp)
    }
    
    fn parse_color(color_str: &str) -> Option<Color> {
        // Check if it's a hex color (starts with #)
        if color_str.starts_with('#') {
            return Self::parse_hex_color(color_str);
        }
        
        // Otherwise, parse as named color
        match color_str.to_lowercase().as_str() {
            "black" => Some(Color::FG_BLACK),
            "red" => Some(Color::FG_RED),
            "green" => Some(Color::FG_GREEN),
            "yellow" => Some(Color::FG_YELLOW),
            "blue" => Some(Color::FG_BLUE),
            "magenta" => Some(Color::FG_MAGENTA),
            "cyan" => Some(Color::FG_CYAN),
            "white" => Some(Color::FG_WHITE),
            _ => {
                eprintln!("Warning: Unknown color '{}', using default. Use color names or hex codes like #ff5733", color_str);
                None
            }
        }
    }
    
    fn parse_hex_color(hex: &str) -> Option<Color> {
        // Remove the # prefix
        let hex = hex.trim_start_matches('#');
        
        // Parse 6-digit hex color (RRGGBB)
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                // Build RGB color using format
                return Some(Color::new(format!("\x1b[38;2;{};{};{}m", r, g, b), format!("\x1b[39m")));
            }
        }
        
        // Parse 3-digit hex color (RGB) - shorthand
        if hex.len() == 3 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..1], 16),
                u8::from_str_radix(&hex[1..2], 16),
                u8::from_str_radix(&hex[2..3], 16),
            ) {
                // Expand shorthand (e.g., #f00 -> #ff0000)
                let (r, g, b) = (r * 17, g * 17, b * 17); // 0xF -> 0xFF (15 * 17 = 255)
                return Some(Color::new(format!("\x1b[38;2;{};{};{}m", r, g, b), format!("\x1b[39m")));
            }
        }
        
        eprintln!("Warning: Invalid hex color '{}', expected format #RRGGBB or #RGB", hex);
        None
    }
}
