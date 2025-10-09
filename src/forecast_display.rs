use crate::weather::ForecastData;

pub struct ForecastFormatter;

impl ForecastFormatter {
    pub fn format(forecast: &ForecastData) -> String {
        let mut output = String::new();
        
        // Get terminal width, default to 80 if unable to detect
        let term_width = terminal_size::terminal_size()
            .map(|(w, _)| w.0 as usize)
            .unwrap_or(80);
        
        // Calculate column width based on terminal width and number of days
        // Leave space for separators (1 space between columns)
        let num_days = forecast.days.len();
        let separator_space = num_days.saturating_sub(1); // spaces between columns
        let available_width = term_width.saturating_sub(separator_space);
        let col_width = (available_width / num_days).max(16).min(24); // min 16, max 24
        
        // Calculate description max length based on column width
        let desc_max_len = col_width.saturating_sub(4); // Account for padding
        
        // Header
        output.push_str(&format!("\n📍 5-Day Forecast for {}\n", forecast.location));
        output.push_str(&format!("{}\n\n", "═".repeat(term_width.min(col_width * num_days + separator_space))));
        
        // Day names row
        let day_names: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(&day.date.format("%a %m/%d").to_string(), col_width))
            .collect();
        output.push_str(&format!("{}\n", day_names.join(" ")));
        
        // Weather icons row
        let icons: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(&format!(" {} ", Self::get_weather_icon(&day.description)), col_width))
            .collect();
        output.push_str(&format!("{}\n", icons.join(" ")));
        
        // Description row
        let descriptions: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(&Self::truncate(&day.description, desc_max_len), col_width))
            .collect();
        output.push_str(&format!("{}\n\n", descriptions.join(" ")));
        
        // Temperature row
        let temps: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(
                &format!("{}°-{}°", Self::format_temp(day.temp_min), Self::format_temp(day.temp_max)),
                col_width
            ))
            .collect();
        output.push_str(&format!("{}\n", temps.join(" ")));
        
        // Humidity row
        let humidity: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(&format!("💧 {}%", day.humidity as i32), col_width))
            .collect();
        output.push_str(&format!("{}\n", humidity.join(" ")));
        
        // Wind row
        let wind: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(&format!("💨 {:.1}m/s", day.wind_speed), col_width))
            .collect();
        output.push_str(&format!("{}\n", wind.join(" ")));
        
        // Precipitation row
        let precip: Vec<String> = forecast.days.iter()
            .map(|day| Self::pad_center(
                &format!("☔ {}%", (day.precipitation_probability * 100.0) as i32),
                col_width
            ))
            .collect();
        output.push_str(&format!("{}\n", precip.join(" ")));
        
        output.push_str(&format!("\n{}\n", "═".repeat(term_width.min(col_width * num_days + separator_space))));
        output
    }
    
    fn truncate(text: &str, max_len: usize) -> String {
        if text.len() > max_len {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        } else {
            text.to_string()
        }
    }
    
    fn pad_center(text: &str, width: usize) -> String {
        let visible_len = Self::visible_length(text);
        
        if visible_len >= width {
            return text.to_string();
        }
        
        let padding = width - visible_len;
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        
        format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
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
}
