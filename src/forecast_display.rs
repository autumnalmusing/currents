use crate::weather::ForecastData;
use comfy_table::*;
use comfy_table::presets::UTF8_FULL;

pub struct ForecastFormatter;

impl ForecastFormatter {
    pub fn format(forecast: &ForecastData) -> String {
        let mut table = Table::new();
        
        // Set up table styling
        table.load_preset(UTF8_FULL)
             .set_content_arrangement(ContentArrangement::Dynamic);
        
        // Set header with day names
        let header: Vec<Cell> = forecast.days.iter()
            .map(|day| {
                Cell::new(day.date.format("%a %m/%d"))
                    .set_alignment(CellAlignment::Center)
            })
            .collect();
        table.set_header(header);
        
        // Weather icon + description row
        let weather_row: Vec<Cell> = forecast.days.iter()
            .map(|day| {
                let icon = Self::get_weather_icon(&day.description);
                Cell::new(format!("{}\n{}", icon, day.description))
                    .set_alignment(CellAlignment::Center)
            })
            .collect();
        table.add_row(weather_row);
        
        // Temperature row
        let temp_row: Vec<Cell> = forecast.days.iter()
            .map(|day| {
                Cell::new(format!("🌡️  {}°-{}°", 
                    Self::format_temp(day.temp_min), 
                    Self::format_temp(day.temp_max)))
                    .set_alignment(CellAlignment::Center)
            })
            .collect();
        table.add_row(temp_row);
        
        // Humidity row
        let humidity_row: Vec<Cell> = forecast.days.iter()
            .map(|day| {
                Cell::new(format!("💧 {}%", day.humidity as i32))
                    .set_alignment(CellAlignment::Center)
            })
            .collect();
        table.add_row(humidity_row);
        
        // Wind row
        let wind_row: Vec<Cell> = forecast.days.iter()
            .map(|day| {
                Cell::new(format!("💨 {:.1}m/s", day.wind_speed))
                    .set_alignment(CellAlignment::Center)
            })
            .collect();
        table.add_row(wind_row);
        
        // Precipitation row
        let precip_row: Vec<Cell> = forecast.days.iter()
            .map(|day| {
                Cell::new(format!("☔ {}%", (day.precipitation_probability * 100.0) as i32))
                    .set_alignment(CellAlignment::Center)
            })
            .collect();
        table.add_row(precip_row);
        
        format!("\n📍 5-Day Forecast for {}\n{}\n", forecast.location, table)
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
}
