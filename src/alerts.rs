use crate::config::{AlertRule, WeatherCondition, TemperatureRange, Range, PrecipitationCondition};
use crate::weather::WeatherData;
use tracing::info;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct AlertEngine {
    rules: Vec<AlertRule>,
    state: AlertState,
}

#[derive(Debug, Clone)]
struct AlertState {
    // Maps rule name to the last time it was triggered
    last_triggered: HashMap<String, u64>,
    // Maps rule name to whether the condition is currently active
    currently_active: HashMap<String, bool>,
}

impl AlertEngine {
    pub fn new(rules: Vec<AlertRule>) -> Self {
        Self { 
            rules,
            state: AlertState {
                last_triggered: HashMap::new(),
                currently_active: HashMap::new(),
            }
        }
    }
    
    pub fn check_alerts(&mut self, weather: &WeatherData) -> Vec<TriggeredAlert> {
        let mut triggered = Vec::new();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }
            
            let condition_matches = self.matches_condition(&rule.condition, weather);
            let was_active = self.state.currently_active.get(&rule.name).copied().unwrap_or(false);
            
            // Update current state
            self.state.currently_active.insert(rule.name.clone(), condition_matches);
            
            if condition_matches {
                let should_notify = self.should_send_alert(&rule, was_active, now);
                
                if should_notify {
                    info!("Alert triggered: {}", rule.name);
                    self.state.last_triggered.insert(rule.name.clone(), now);
                    triggered.push(TriggeredAlert {
                        rule_name: rule.name.clone(),
                        message: rule.message.clone(),
                    });
                }
            }
        }
        
        triggered
    }
    
    fn should_send_alert(&self, rule: &AlertRule, was_active: bool, now: u64) -> bool {
        match rule.repeat.as_str() {
            "always" => {
                // Always send the alert, every time the condition is checked
                true
            }
            "once" => {
                // Only send if the condition was not previously active (edge-triggered)
                !was_active
            }
            repeat_str => {
                // Try to parse as duration in seconds
                if let Ok(seconds) = repeat_str.parse::<u64>() {
                    // Check if enough time has passed since last alert
                    if let Some(&last_time) = self.state.last_triggered.get(&rule.name) {
                        now - last_time >= seconds
                    } else {
                        // Never triggered before
                        true
                    }
                } else {
                    // Invalid format, default to "once" behavior
                    !was_active
                }
            }
        }
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
