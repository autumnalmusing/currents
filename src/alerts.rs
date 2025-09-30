use crate::config::{AlertRule, WeatherCondition, TemperatureRange, Range, PrecipitationCondition};
use crate::weather::WeatherData;
use tracing::info;

#[derive(Debug, Clone)]
pub struct AlertEngine {
    rules: Vec<AlertRule>,
}

impl AlertEngine {
    pub fn new(rules: Vec<AlertRule>) -> Self {
        Self { rules }
    }
    
    pub fn check_alerts(&self, weather: &WeatherData) -> Vec<TriggeredAlert> {
        let mut triggered = Vec::new();
        
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            
            if self.matches_condition(&rule.condition, weather) {
                info!("Alert triggered: {}", rule.name);
                triggered.push(TriggeredAlert {
                    rule_name: rule.name.clone(),
                    message: rule.message.clone(),
                });
            }
        }
        
        triggered
    }
    
    fn matches_condition(&self, condition: &WeatherCondition, weather: &WeatherData) -> bool {
        // Check temperature conditions
        if let Some(ref temp_range) = condition.temperature {
            if !self.check_temperature_range(temp_range, weather.temperature) {
                return false;
            }
        }
        
        // Check humidity conditions
        if let Some(ref humidity_range) = condition.humidity {
            if !self.check_range(humidity_range, weather.humidity) {
                return false;
            }
        }
        
        // Check wind speed conditions
        if let Some(ref wind_range) = condition.wind_speed {
            if !self.check_range(wind_range, weather.wind_speed) {
                return false;
            }
        }
        
        // Check precipitation conditions
        if let Some(ref precip_condition) = condition.precipitation {
            if !self.check_precipitation_condition(precip_condition, weather) {
                return false;
            }
        }
        
        // Check description keywords
        if let Some(ref description_keywords) = condition.description {
            if !self.check_description_keywords(description_keywords, &weather.description) {
                return false;
            }
        }
        
        true
    }
    
    fn check_temperature_range(&self, range: &TemperatureRange, temperature: f64) -> bool {
        if let Some(min) = range.min {
            if temperature < min {
                return false;
            }
        }
        
        if let Some(max) = range.max {
            if temperature > max {
                return false;
            }
        }
        
        true
    }
    
    fn check_range(&self, range: &Range, value: f64) -> bool {
        if let Some(min) = range.min {
            if value < min {
                return false;
            }
        }
        
        if let Some(max) = range.max {
            if value > max {
                return false;
            }
        }
        
        true
    }
    
    fn check_precipitation_condition(&self, condition: &PrecipitationCondition, weather: &WeatherData) -> bool {
        if let Some(ref precip_data) = weather.precipitation {
            // Check intensity
            if let Some(ref required_intensity) = condition.intensity {
                if !precip_data.intensity.to_lowercase().contains(&required_intensity.to_lowercase()) {
                    return false;
                }
            }
            
            // Check probability
            if let Some(required_probability) = condition.probability {
                if precip_data.probability < required_probability {
                    return false;
                }
            }
        } else {
            // No precipitation data available
            return false;
        }
        
        true
    }
    
    fn check_description_keywords(&self, keywords: &str, description: &str) -> bool {
        let description_lower = description.to_lowercase();
        let keywords_lower = keywords.to_lowercase();
        
        // Support multiple keywords separated by commas or spaces
        let keyword_list: Vec<&str> = keywords_lower
            .split(&[',', ' ', '|'][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        for keyword in keyword_list {
            if description_lower.contains(keyword) {
                return true;
            }
        }
        
        false
    }
}

#[derive(Debug, Clone)]
pub struct TriggeredAlert {
    pub rule_name: String,
    pub message: String,
}

impl TriggeredAlert {
    pub fn new(rule_name: String, message: String) -> Self {
        Self { rule_name, message }
    }
}
