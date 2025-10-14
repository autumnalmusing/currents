//! Configuration management for the orchestrator

use crate::types::{LocationConfig, OrchestratorConfig, WeatherConfig, CollectionStrategy};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// TOML configuration structure for the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfigToml {
    pub orchestrator: OrchestratorSection,
    pub locations: HashMap<String, LocationSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorSection {
    pub storage_path: Option<String>,
    pub analysis_interval: Option<u64>,
    pub health_check_interval: Option<u64>,
    pub batch_size: Option<usize>,
    pub global_api_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationSection {
    pub name: String,
    pub coordinates: [f64; 2], // [latitude, longitude]
    pub weather: WeatherSection,
    pub collection: CollectionSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherSection {
    pub api_key: String,
    pub provider: String,
    pub units: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionSection {
    pub interval: u64,
    pub strategy: String,
}

impl OrchestratorConfig {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;
        
        let toml_config: OrchestratorConfigToml = toml::from_str(&content)
            .with_context(|| "Failed to parse TOML configuration")?;
        
        toml_config.try_into()
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_config: OrchestratorConfigToml = self.into();
        let content = toml::to_string_pretty(&toml_config)
            .context("Failed to serialize configuration to TOML")?;
        
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;
        
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.locations.is_empty() {
            return Err(anyhow::anyhow!("No locations configured"));
        }

        for (location_id, location) in &self.locations {
            if location.coordinates.0 < -90.0 || location.coordinates.0 > 90.0 {
                return Err(anyhow::anyhow!(
                    "Invalid latitude for location '{}': {}",
                    location_id,
                    location.coordinates.0
                ));
            }

            if location.coordinates.1 < -180.0 || location.coordinates.1 > 180.0 {
                return Err(anyhow::anyhow!(
                    "Invalid longitude for location '{}': {}",
                    location_id,
                    location.coordinates.1
                ));
            }

            if location.weather_config.api_key.is_empty() {
                return Err(anyhow::anyhow!(
                    "API key is required for location '{}'",
                    location_id
                ));
            }
        }

        Ok(())
    }
}

impl TryFrom<OrchestratorConfigToml> for OrchestratorConfig {
    type Error = anyhow::Error;

    fn try_from(toml: OrchestratorConfigToml) -> Result<Self> {
        let mut locations = HashMap::new();

        for (location_id, location_section) in toml.locations {
            let collection_strategy = match location_section.collection.strategy.as_str() {
                "interval" => CollectionStrategy::Interval,
                "threshold" => CollectionStrategy::Threshold,
                "hybrid" => CollectionStrategy::Hybrid,
                _ => return Err(anyhow::anyhow!(
                    "Invalid collection strategy for location '{}': {}",
                    location_id,
                    location_section.collection.strategy
                )),
            };

            let location_config = LocationConfig {
                id: location_id.clone(),
                name: location_section.name,
                coordinates: (location_section.coordinates[0], location_section.coordinates[1]),
                weather_config: WeatherConfig {
                    api_key: location_section.weather.api_key,
                    provider: location_section.weather.provider,
                    units: location_section.weather.units.unwrap_or_else(|| "metric".to_string()),
                    collection_interval: location_section.collection.interval,
                },
                collection_strategy,
            };

            locations.insert(location_id, location_config);
        }

        Ok(OrchestratorConfig {
            storage_path: toml.orchestrator.storage_path
                .unwrap_or_else(|| "~/.config/currents/orchestrator.db".to_string()),
            analysis_interval: toml.orchestrator.analysis_interval.unwrap_or(3600),
            health_check_interval: toml.orchestrator.health_check_interval.unwrap_or(300),
            batch_size: toml.orchestrator.batch_size,
            global_api_limit: toml.orchestrator.global_api_limit,
            locations,
        })
    }
}

impl From<&OrchestratorConfig> for OrchestratorConfigToml {
    fn from(config: &OrchestratorConfig) -> Self {
        let mut locations = HashMap::new();

        for (location_id, location) in &config.locations {
            let strategy_str = match location.collection_strategy {
                CollectionStrategy::Interval => "interval",
                CollectionStrategy::Threshold => "threshold",
                CollectionStrategy::Hybrid => "hybrid",
            };

            let location_section = LocationSection {
                name: location.name.clone(),
                coordinates: [location.coordinates.0, location.coordinates.1],
                weather: WeatherSection {
                    api_key: location.weather_config.api_key.clone(),
                    provider: location.weather_config.provider.clone(),
                    units: Some(location.weather_config.units.clone()),
                },
                collection: CollectionSection {
                    interval: location.weather_config.collection_interval,
                    strategy: strategy_str.to_string(),
                },
            };

            locations.insert(location_id.clone(), location_section);
        }

        OrchestratorConfigToml {
            orchestrator: OrchestratorSection {
                storage_path: Some(config.storage_path.clone()),
                analysis_interval: Some(config.analysis_interval),
                health_check_interval: Some(config.health_check_interval),
                batch_size: config.batch_size,
                global_api_limit: config.global_api_limit,
            },
            locations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_config_validation() {
        let mut config = OrchestratorConfig::default();
        
        // Empty locations should fail
        assert!(config.validate().is_err());

        // Add a valid location
        config.locations.insert("london".to_string(), LocationConfig {
            id: "london".to_string(),
            name: "London, UK".to_string(),
            coordinates: (51.5074, -0.1278),
            weather_config: WeatherConfig {
                api_key: "test-key".to_string(),
                provider: "openweathermap".to_string(),
                units: "metric".to_string(),
                collection_interval: 1800,
            },
            collection_strategy: CollectionStrategy::Interval,
        });

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_coordinates() {
        let mut config = OrchestratorConfig::default();
        
        // Invalid latitude
        config.locations.insert("invalid".to_string(), LocationConfig {
            id: "invalid".to_string(),
            name: "Invalid Location".to_string(),
            coordinates: (91.0, 0.0), // Invalid latitude
            weather_config: WeatherConfig {
                api_key: "test-key".to_string(),
                provider: "openweathermap".to_string(),
                units: "metric".to_string(),
                collection_interval: 1800,
            },
            collection_strategy: CollectionStrategy::Interval,
        });

        assert!(config.validate().is_err());
    }
}