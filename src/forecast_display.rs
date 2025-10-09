use crate::weather::ForecastData;

pub struct ForecastFormatter;

impl ForecastFormatter {
    pub fn format(forecast: &ForecastData) -> String {
        let mut output = String::new();
        let col_width = 16;
        
        // Header
        output.push_str(&format!("\n📍 5-Day Forecast for {}\n", forecast.location));
        output.push_str(&format!("{}\n\n", "═".repeat(col_width * 5 + 4)));
        
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
            .map(|day| Self::pad_center(&Self::truncate(&day.description, 14), col_width))
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
        
        output.push_str(&format!("\n{}\n", "═".repeat(col_width * 5 + 4)));
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
