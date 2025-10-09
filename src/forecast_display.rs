use crate::weather::{ForecastData, ForecastDay};

pub struct ForecastFormatter;

impl ForecastFormatter {
    pub fn format(forecast: &ForecastData) -> String {
        let mut output = String::new();
        
        // Header
        output.push_str(&format!("\n{}\n", Self::center_text("╔══════════════════════════════════════════════════════════════╗", 62)));
        output.push_str(&format!("{}\n", Self::center_text(&format!("║  📍 5-Day Forecast for {}  ║", Self::pad_location(&forecast.location)), 62)));
        output.push_str(&format!("{}\n\n", Self::center_text("╚══════════════════════════════════════════════════════════════╝", 62)));
        
        // Days
        for (idx, day) in forecast.days.iter().enumerate() {
            if idx > 0 {
                let separator = "─".repeat(58);
                output.push_str(&format!("{}\n", Self::center_text(&separator, 62)));
            }
            output.push_str(&Self::format_day(day));
        }
        
        let bottom_border = "═".repeat(58);
        output.push_str(&format!("\n{}\n", Self::center_text(&bottom_border, 62)));
        output
    }
    
    fn format_day(day: &ForecastDay) -> String {
        let mut output = String::new();
        
        // Date and day of week
        let day_name = day.date.format("%A").to_string();
        let date_str = day.date.format("%B %d, %Y").to_string();
        let icon = Self::get_weather_icon(&day.description);
        
        output.push_str(&format!("{}\n", Self::center_text(&format!("  {}  {}  ", day_name, icon), 62)));
        output.push_str(&format!("{}\n\n", Self::center_text(&date_str, 62)));
        
        // Weather description
        output.push_str(&format!("{}\n", Self::center_text(&format!("  {}  ", day.description), 62)));
        
        // Temperature (centered and formatted)
        let temp_str = format!("🌡️  {}°C - {}°C", 
            Self::format_temp(day.temp_min), 
            Self::format_temp(day.temp_max)
        );
        output.push_str(&format!("{}\n", Self::center_text(&temp_str, 62)));
        
        // Additional details
        let details = format!(
            "💧 {}% humidity  |  💨 {:.1} m/s wind  |  ☔ {}% precip",
            day.humidity as i32,
            day.wind_speed,
            (day.precipitation_probability * 100.0) as i32
        );
        output.push_str(&format!("{}\n\n", Self::center_text(&details, 62)));
        
        output
    }
    
    fn get_weather_icon(description: &str) -> &'static str {
        let desc_lower = description.to_lowercase();
        
        if desc_lower.contains("clear") {
            "☀️"
        } else if desc_lower.contains("sun") {
            "☀️"
        } else if desc_lower.contains("cloud") && desc_lower.contains("few") {
            "🌤️"
        } else if desc_lower.contains("cloud") && (desc_lower.contains("scattered") || desc_lower.contains("partly")) {
            "⛅"
        } else if desc_lower.contains("cloud") {
            "☁️"
        } else if desc_lower.contains("overcast") {
            "☁️"
        } else if desc_lower.contains("rain") && desc_lower.contains("heavy") {
            "🌧️"
        } else if desc_lower.contains("rain") && desc_lower.contains("light") {
            "🌦️"
        } else if desc_lower.contains("rain") {
            "🌧️"
        } else if desc_lower.contains("drizzle") {
            "🌦️"
        } else if desc_lower.contains("thunder") || desc_lower.contains("storm") {
            "⛈️"
        } else if desc_lower.contains("snow") {
            "❄️"
        } else if desc_lower.contains("mist") || desc_lower.contains("fog") {
            "🌫️"
        } else if desc_lower.contains("haze") {
            "🌫️"
        } else {
            "🌈"
        }
    }
    
    fn format_temp(temp: f64) -> String {
        format!("{:.0}", temp)
    }
    
    fn center_text(text: &str, width: usize) -> String {
        // Count visible characters (excluding ANSI codes and emojis count as 1)
        let visible_len = Self::visible_length(text);
        
        if visible_len >= width {
            return text.to_string();
        }
        
        let padding = width - visible_len;
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        
        format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
    }
    
    fn visible_length(text: &str) -> usize {
        // Simple approximation: count chars but treat emoji sequences specially
        let mut len = 0;
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch as u32 >= 0x1F000 {
                // Emoji range, count as 2 for display width
                len += 2;
                // Skip variation selectors and zero-width joiners
                while let Some(&next_ch) = chars.peek() {
                    if (next_ch as u32) == 0xFE0F || (next_ch as u32) == 0x200D {
                        chars.next();
                    } else {
                        break;
                    }
                }
            } else {
                len += 1;
            }
        }
        
        len
    }
    
    fn pad_location(location: &str) -> String {
        // Pad or truncate location to fit nicely in header
        let max_len = 40;
        if location.len() > max_len {
            format!("{}...", &location[..max_len - 3])
        } else {
            location.to_string()
        }
    }
}

