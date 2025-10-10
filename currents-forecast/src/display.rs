use currents_core::ForecastData;
use crate::config::{ForecastHighlights, ForecastDisplayConfig};
use tabled::{Table, settings::{Style, Alignment, Modify, object::Columns, Color}, builder::Builder};

pub struct ForecastFormatter;

impl ForecastFormatter {
    pub fn format(forecast: &ForecastData, highlights: &ForecastHighlights, display_config: &ForecastDisplayConfig) -> String {
        let mut builder = Builder::default();
        
        // Build header row
        let mut headers = Vec::new();
        if display_config.show_date {
            headers.push("Date");
        }
        if display_config.show_weather {
            headers.push("Weather");
        }
        if display_config.show_temp {
            headers.push("Temp");
        }
        if display_config.show_humidity {
            headers.push("Humidity");
        }
        if display_config.show_wind {
            headers.push("Wind");
        }
        if display_config.show_precip {
            headers.push("Precip");
        }
        if display_config.show_pressure {
            headers.push("Pressure");
        }
        if display_config.show_visibility {
            headers.push("Visibility");
        }
        if display_config.show_uv {
            headers.push("UV");
        }
        if display_config.show_clouds {
            headers.push("Clouds");
        }
        if display_config.show_wind_dir {
            headers.push("Wind Dir");
        }
        if display_config.show_aqi {
            headers.push("AQI");
        }
        builder.push_record(headers);
        
        // Build data rows
        for day in &forecast.days {
            let mut row = Vec::new();
            
            if display_config.show_date {
                row.push(day.date.format("%a %m/%d").to_string());
            }
            if display_config.show_weather {
                row.push(day.description.clone());
            }
            if display_config.show_temp {
                row.push(format!("{}°-{}°", Self::format_temp(day.temp_min), Self::format_temp(day.temp_max)));
            }
            if display_config.show_humidity {
                row.push(format!("{}%", day.humidity as i32));
            }
            if display_config.show_wind {
                row.push(format!("{:.1}m/s", day.wind_speed));
            }
            if display_config.show_precip {
                row.push(format!("{}%", (day.precipitation_probability * 100.0) as i32));
            }
            if display_config.show_pressure {
                row.push(day.pressure.map_or("N/A".to_string(), |p| format!("{:.0}mb", p)));
            }
            if display_config.show_visibility {
                row.push(day.visibility.map_or("N/A".to_string(), |v| format!("{:.1}km", v)));
            }
            if display_config.show_uv {
                row.push(day.uv_index.map_or("N/A".to_string(), |u| format!("{:.1}", u)));
            }
            if display_config.show_clouds {
                row.push(day.cloud_cover.map_or("N/A".to_string(), |c| format!("{}%", c as i32)));
            }
            if display_config.show_wind_dir {
                row.push(day.wind_direction.clone().unwrap_or_else(|| "N/A".to_string()));
            }
            if display_config.show_aqi {
                row.push(day.aqi.map_or("N/A".to_string(), |a| Self::format_aqi(a)));
            }
            
            builder.push_record(row);
        }
        
        let mut table = builder.build();
        table.with(Style::rounded())
             .with(Modify::new(Columns::new(..)).with(Alignment::center()));
        
        // Apply color highlights after table is built
        Self::apply_highlights(&mut table, forecast, highlights, display_config);
        
        format!("\n5-Day Forecast for {}\n{}\n", forecast.location, table)
    }
    
    fn apply_highlights(table: &mut Table, forecast: &ForecastData, highlights: &ForecastHighlights, display_config: &ForecastDisplayConfig) {
        for (row_idx, day) in forecast.days.iter().enumerate() {
            let table_row = row_idx + 1; // +1 because row 0 is the header
            let mut col_idx = 0;
            
            // Date column (no highlighting)
            if display_config.show_date {
                col_idx += 1;
            }
            
            // Weather column (no highlighting)
            if display_config.show_weather {
                col_idx += 1;
            }
            
            // Temperature highlighting
            if display_config.show_temp {
                if let Some(high) = highlights.temperature.high {
                    if day.temp_max >= high {
                        if let Some(color) = Self::parse_color(&highlights.temperature.high_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                if let Some(low) = highlights.temperature.low {
                    if day.temp_min <= low {
                        if let Some(color) = Self::parse_color(&highlights.temperature.low_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                col_idx += 1;
            }
            
            // Humidity highlighting
            if display_config.show_humidity {
                if let Some(high) = highlights.humidity.high {
                    if day.humidity >= high {
                        if let Some(color) = Self::parse_color(&highlights.humidity.high_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                if let Some(low) = highlights.humidity.low {
                    if day.humidity <= low {
                        if let Some(color) = Self::parse_color(&highlights.humidity.low_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                col_idx += 1;
            }
            
            // Wind speed highlighting
            if display_config.show_wind {
                if let Some(high) = highlights.wind_speed.high {
                    if day.wind_speed >= high {
                        if let Some(color) = Self::parse_color(&highlights.wind_speed.high_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                if let Some(low) = highlights.wind_speed.low {
                    if day.wind_speed <= low {
                        if let Some(color) = Self::parse_color(&highlights.wind_speed.low_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                col_idx += 1;
            }
            
            // Precipitation highlighting
            if display_config.show_precip {
                let precip_percent = day.precipitation_probability * 100.0;
                if let Some(high) = highlights.precipitation.high {
                    if precip_percent >= high {
                        if let Some(color) = Self::parse_color(&highlights.precipitation.high_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                if let Some(low) = highlights.precipitation.low {
                    if precip_percent <= low {
                        if let Some(color) = Self::parse_color(&highlights.precipitation.low_color) {
                            table.with(Modify::new((table_row, col_idx)).with(color));
                        }
                    }
                }
                col_idx += 1;
            }
            
            // Pressure highlighting
            if display_config.show_pressure {
                if let Some(pressure) = day.pressure {
                    if let Some(high) = highlights.pressure.high {
                        if pressure >= high {
                            if let Some(color) = Self::parse_color(&highlights.pressure.high_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                    if let Some(low) = highlights.pressure.low {
                        if pressure <= low {
                            if let Some(color) = Self::parse_color(&highlights.pressure.low_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                }
                col_idx += 1;
            }
            
            // Visibility highlighting
            if display_config.show_visibility {
                if let Some(visibility) = day.visibility {
                    if let Some(high) = highlights.visibility.high {
                        if visibility >= high {
                            if let Some(color) = Self::parse_color(&highlights.visibility.high_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                    if let Some(low) = highlights.visibility.low {
                        if visibility <= low {
                            if let Some(color) = Self::parse_color(&highlights.visibility.low_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                }
                col_idx += 1;
            }
            
            // UV Index highlighting
            if display_config.show_uv {
                if let Some(uv) = day.uv_index {
                    if let Some(high) = highlights.uv_index.high {
                        if uv >= high {
                            if let Some(color) = Self::parse_color(&highlights.uv_index.high_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                    if let Some(low) = highlights.uv_index.low {
                        if uv <= low {
                            if let Some(color) = Self::parse_color(&highlights.uv_index.low_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                }
                col_idx += 1;
            }
            
            // Cloud cover highlighting
            if display_config.show_clouds {
                if let Some(clouds) = day.cloud_cover {
                    if let Some(high) = highlights.cloud_cover.high {
                        if clouds >= high {
                            if let Some(color) = Self::parse_color(&highlights.cloud_cover.high_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                    if let Some(low) = highlights.cloud_cover.low {
                        if clouds <= low {
                            if let Some(color) = Self::parse_color(&highlights.cloud_cover.low_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                }
                // Note: col_idx not incremented here as we're at the end of the loop
            }
            
            // Wind direction column (no highlighting)
            if display_config.show_wind_dir {
                // Wind direction is just text, no highlighting needed
                col_idx += 1;
            }
            
            // AQI highlighting
            if display_config.show_aqi {
                if let Some(aqi) = day.aqi {
                    if let Some(high) = highlights.aqi.high {
                        if aqi >= high {
                            if let Some(color) = Self::parse_color(&highlights.aqi.high_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                    if let Some(low) = highlights.aqi.low {
                        if aqi <= low {
                            if let Some(color) = Self::parse_color(&highlights.aqi.low_color) {
                                table.with(Modify::new((table_row, col_idx)).with(color));
                            }
                        }
                    }
                }
            }
        }
    }
    
    fn format_temp(temp: f64) -> String {
        format!("{:.0}", temp)
    }
    
    fn format_aqi(aqi: f64) -> String {
        // US EPA AQI scale: 1=Good, 2=Moderate, 3=Unhealthy for Sensitive, 4=Unhealthy, 5=Very Unhealthy, 6=Hazardous
        let label = match aqi as i32 {
            1 => "Good",
            2 => "Moderate",
            3 => "Sensitive",
            4 => "Unhealthy",
            5 => "Very Bad",
            6 => "Hazardous",
            _ => "Unknown",
        };
        format!("{} ({})", aqi as i32, label)
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
