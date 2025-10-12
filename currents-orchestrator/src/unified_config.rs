//! Unified configuration system for both orchestrator and collectors
//!
//! This module provides a unified configuration system that can read from:
//! - TOML configuration files (for orchestrator)
//! - Environment variables (for collectors)
//! - Command line arguments
//! - Default values

use crate::types::{LocationConfig, WeatherConfig, CollectionStrategy};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Unified configuration that can be loaded from multiple sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedConfig {
    pub orchestrator: OrchestratorConfig,
    pub locations: HashMap<String, LocationConfig>,
}

/// Orchestrator configuration section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub storage_path: String,
    pub analysis_interval: u64,
    pub health_check_interval: u64,
    pub log_level: String,
    pub log_format: String,
}

/// Collector configuration that can be loaded from environment or TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub location_id: String,
    pub location_name: String,
    pub api_key: String,
    pub provider: String,
    pub units: String,
    pub collection_interval: u64,
    pub storage_path: String,
    pub log_level: Option<String>,
    pub log_format: Option<String>,
}

impl UnifiedConfig {
    /// Load configuration from TOML file with environment variable overrides
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let mut config: UnifiedConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
        
        // Apply environment variable overrides
        config.apply_env_overrides();
        
        Ok(config)
    }
    
    /// Load configuration from environment variables (for collectors)
    pub fn from_env() -> Result<Self> {
        let storage_path = std::env::var("ORCHESTRATOR_STORAGE_PATH")
            .unwrap_or_else(|_| "~/.config/currents/orchestrator.db".to_string());
        
        let orchestrator = OrchestratorConfig {
            storage_path: shellexpand::tilde(&storage_path).to_string(),
            analysis_interval: std::env::var("ORCHESTRATOR_ANALYSIS_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            health_check_interval: std::env::var("ORCHESTRATOR_HEALTH_CHECK_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            log_level: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info".to_string()),
            log_format: std::env::var("RUST_LOG_FORMAT")
                .unwrap_or_else(|_| "pretty".to_string()),
        };
        
        let mut locations = HashMap::new();
        
        // Load locations from environment variables
        // Format: LOCATION_<ID>_<FIELD>=<value>
        let mut location_vars: HashMap<String, HashMap<String, String>> = HashMap::new();
        
        for (key, value) in std::env::vars() {
            if let Some(rest) = key.strip_prefix("LOCATION_") {
                if let Some((location_id, field)) = rest.split_once('_') {
                    location_vars
                        .entry(location_id.to_lowercase())
                        .or_insert_with(HashMap::new)
                        .insert(field.to_lowercase(), value);
                }
            }
        }
        
        for (location_id, vars) in location_vars {
            if let Some(location_config) = Self::parse_location_from_env(&location_id, &vars)? {
                locations.insert(location_id, location_config);
            }
        }
        
        Ok(UnifiedConfig {
            orchestrator,
            locations,
        })
    }
    
    /// Apply environment variable overrides to existing configuration
    fn apply_env_overrides(&mut self) {
        // Override orchestrator settings
        if let Ok(storage_path) = std::env::var("ORCHESTRATOR_STORAGE_PATH") {
            self.orchestrator.storage_path = shellexpand::tilde(&storage_path).to_string();
        }
        
        if let Ok(analysis_interval) = std::env::var("ORCHESTRATOR_ANALYSIS_INTERVAL") {
            if let Ok(interval) = analysis_interval.parse() {
                self.orchestrator.analysis_interval = interval;
            }
        }
        
        if let Ok(health_check_interval) = std::env::var("ORCHESTRATOR_HEALTH_CHECK_INTERVAL") {
            if let Ok(interval) = health_check_interval.parse() {
                self.orchestrator.health_check_interval = interval;
            }
        }
        
        if let Ok(log_level) = std::env::var("RUST_LOG") {
            self.orchestrator.log_level = log_level;
        }
        
        if let Ok(log_format) = std::env::var("RUST_LOG_FORMAT") {
            self.orchestrator.log_format = log_format;
        }
        
        // Override location settings
        for (location_id, location_config) in &mut self.locations {
            let env_prefix = format!("LOCATION_{}_", location_id.to_uppercase());
            
            if let Ok(name) = std::env::var(&format!("{}NAME", env_prefix)) {
                location_config.name = name;
            }
            
            if let Ok(coords) = std::env::var(&format!("{}COORDINATES", env_prefix)) {
                if let Some((lat, lon)) = coords.split_once(',') {
                    if let (Ok(lat), Ok(lon)) = (lat.trim().parse(), lon.trim().parse()) {
                        location_config.coordinates = (lat, lon);
                    }
                }
            }
            
            if let Ok(api_key) = std::env::var(&format!("{}API_KEY", env_prefix)) {
                location_config.weather_config.api_key = api_key;
            }
            
            if let Ok(provider) = std::env::var(&format!("{}PROVIDER", env_prefix)) {
                location_config.weather_config.provider = provider;
            }
            
            if let Ok(units) = std::env::var(&format!("{}UNITS", env_prefix)) {
                location_config.weather_config.units = units;
            }
            
            if let Ok(interval) = std::env::var(&format!("{}INTERVAL", env_prefix)) {
                if let Ok(interval) = interval.parse() {
                    location_config.weather_config.collection_interval = interval;
                }
            }
        }
    }
    
    /// Parse a location configuration from environment variables
    fn parse_location_from_env(
        location_id: &str,
        vars: &HashMap<String, String>,
    ) -> Result<Option<LocationConfig>> {
        let name = vars.get("name")
            .ok_or_else(|| anyhow::anyhow!("Missing LOCATION_{}_NAME", location_id.to_uppercase()))?;
        
        let coordinates = if let Some(coords) = vars.get("coordinates") {
            if let Some((lat, lon)) = coords.split_once(',') {
                (lat.trim().parse()?, lon.trim().parse()?)
            } else {
                return Err(anyhow::anyhow!("Invalid coordinates format for {}", location_id));
            }
        } else {
            return Err(anyhow::anyhow!("Missing LOCATION_{}_COORDINATES", location_id.to_uppercase()));
        };
        
        let api_key = vars.get("api_key")
            .ok_or_else(|| anyhow::anyhow!("Missing LOCATION_{}_API_KEY", location_id.to_uppercase()))?;
        
        let provider = vars.get("provider")
            .cloned()
            .unwrap_or_else(|| "openweathermap".to_string());
        
        let units = vars.get("units")
            .cloned()
            .unwrap_or_else(|| "metric".to_string());
        
        let collection_interval = vars.get("interval")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1800);
        
        let collection_strategy = vars.get("strategy")
            .and_then(|s| match s.as_str() {
                "interval" => Some(CollectionStrategy::Interval),
                "threshold" => Some(CollectionStrategy::Threshold),
                "hybrid" => Some(CollectionStrategy::Hybrid),
                _ => None,
            })
            .unwrap_or(CollectionStrategy::Interval);
        
        Ok(Some(LocationConfig {
            id: location_id.to_string(),
            name: name.clone(),
            coordinates,
            weather_config: WeatherConfig {
                api_key: api_key.clone(),
                provider,
                units,
                collection_interval,
            },
            collection_strategy,
        }))
    }
    
    /// Get configuration for a specific location
    pub fn get_location_config(&self, location_id: &str) -> Option<&LocationConfig> {
        self.locations.get(location_id)
    }
    
    /// Get all location IDs
    pub fn get_location_ids(&self) -> Vec<String> {
        self.locations.keys().cloned().collect()
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate orchestrator settings
        if self.orchestrator.storage_path.is_empty() {
            return Err(anyhow::anyhow!("Storage path cannot be empty"));
        }
        
        if self.orchestrator.analysis_interval == 0 {
            return Err(anyhow::anyhow!("Analysis interval must be greater than 0"));
        }
        
        if self.orchestrator.health_check_interval == 0 {
            return Err(anyhow::anyhow!("Health check interval must be greater than 0"));
        }
        
        // Validate locations
        for (location_id, location) in &self.locations {
            if location.name.is_empty() {
                return Err(anyhow::anyhow!("Location {} name cannot be empty", location_id));
            }
            
            if location.coordinates.0 < -90.0 || location.coordinates.0 > 90.0 {
                return Err(anyhow::anyhow!("Invalid latitude for location {}: {}", location_id, location.coordinates.0));
            }
            
            if location.coordinates.1 < -180.0 || location.coordinates.1 > 180.0 {
                return Err(anyhow::anyhow!("Invalid longitude for location {}: {}", location_id, location.coordinates.1));
            }
            
            if location.weather_config.api_key.is_empty() {
                return Err(anyhow::anyhow!("API key cannot be empty for location {}", location_id));
            }
            
            if location.weather_config.collection_interval == 0 {
                return Err(anyhow::anyhow!("Collection interval must be greater than 0 for location {}", location_id));
            }
        }
        
        Ok(())
    }
}

impl CollectorConfig {
    /// Load collector configuration from environment variables
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            location_id: std::env::var("COLLECTOR_LOCATION_ID")
                .context("COLLECTOR_LOCATION_ID not set")?,
            location_name: std::env::var("COLLECTOR_LOCATION_NAME")
                .context("COLLECTOR_LOCATION_NAME not set")?,
            api_key: std::env::var("COLLECTOR_API_KEY")
                .context("COLLECTOR_API_KEY not set")?,
            provider: std::env::var("COLLECTOR_PROVIDER")
                .unwrap_or_else(|_| "openweathermap".to_string()),
            units: std::env::var("COLLECTOR_UNITS")
                .unwrap_or_else(|_| "metric".to_string()),
            collection_interval: std::env::var("COLLECTOR_INTERVAL")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1800),
            storage_path: std::env::var("COLLECTOR_STORAGE_PATH")
                .context("COLLECTOR_STORAGE_PATH not set")?,
            log_level: std::env::var("RUST_LOG").ok(),
            log_format: std::env::var("RUST_LOG_FORMAT").ok(),
        })
    }
    
    /// Load collector configuration from TOML file with environment overrides
    pub fn from_toml_file<P: AsRef<Path>>(path: P, location_id: &str) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let config: UnifiedConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
        
        let location_config = config.get_location_config(location_id)
            .ok_or_else(|| anyhow::anyhow!("Location {} not found in configuration", location_id))?;
        
        Ok(Self {
            location_id: location_id.to_string(),
            location_name: location_config.name.clone(),
            api_key: location_config.weather_config.api_key.clone(),
            provider: location_config.weather_config.provider.clone(),
            units: location_config.weather_config.units.clone(),
            collection_interval: location_config.weather_config.collection_interval,
            storage_path: config.orchestrator.storage_path.clone(),
            log_level: Some(config.orchestrator.log_level.clone()),
            log_format: Some(config.orchestrator.log_format.clone()),
        })
    }
    
    /// Validate collector configuration
    pub fn validate(&self) -> Result<()> {
        if self.location_id.is_empty() {
            return Err(anyhow::anyhow!("Location ID cannot be empty"));
        }
        
        if self.location_name.is_empty() {
            return Err(anyhow::anyhow!("Location name cannot be empty"));
        }
        
        if self.api_key.is_empty() {
            return Err(anyhow::anyhow!("API key cannot be empty"));
        }
        
        if self.storage_path.is_empty() {
            return Err(anyhow::anyhow!("Storage path cannot be empty"));
        }
        
        if self.collection_interval == 0 {
            return Err(anyhow::anyhow!("Collection interval must be greater than 0"));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_collector_config_from_env() {
        env::set_var("COLLECTOR_LOCATION_ID", "test");
        env::set_var("COLLECTOR_LOCATION_NAME", "Test City");
        env::set_var("COLLECTOR_API_KEY", "test-key");
        env::set_var("COLLECTOR_STORAGE_PATH", "/tmp/test.db");
        
        let config = CollectorConfig::from_env().unwrap();
        assert_eq!(config.location_id, "test");
        assert_eq!(config.location_name, "Test City");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.storage_path, "/tmp/test.db");
        
        // Clean up
        env::remove_var("COLLECTOR_LOCATION_ID");
        env::remove_var("COLLECTOR_LOCATION_NAME");
        env::remove_var("COLLECTOR_API_KEY");
        env::remove_var("COLLECTOR_STORAGE_PATH");
    }
    
    #[test]
    fn test_unified_config_validation() {
        let mut config = UnifiedConfig {
            orchestrator: OrchestratorConfig {
                storage_path: "/tmp/test.db".to_string(),
                analysis_interval: 3600,
                health_check_interval: 300,
                log_level: "info".to_string(),
                log_format: "pretty".to_string(),
            },
            locations: HashMap::new(),
        };
        
        // Should pass validation
        assert!(config.validate().is_ok());
        
        // Test invalid storage path
        config.orchestrator.storage_path = String::new();
        assert!(config.validate().is_err());
    }
}
