//! Region-based collector management
//! 
//! Manages collectors based on logical regions (e.g., "Europe", "Asia", "Americas")
//! for more intuitive organization and resource allocation.

use crate::types::{LocationId, CollectorId, LocationConfig, CollectorStatus};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    pub name: String,
    pub description: Option<String>,
    pub max_locations: usize,
    pub batch_size: Option<usize>, // If None, uses default
    pub priority: u8, // 1-10, higher = more resources
}

#[derive(Debug, Clone)]
pub struct RegionCollector {
    pub id: CollectorId,
    pub region_name: String,
    pub locations: Vec<LocationId>,
    pub status: CollectorStatus,
    pub last_heartbeat: Instant,
    pub process_id: Option<u32>,
    pub batch_size: usize,
}

/// Manages collectors based on logical regions
pub struct RegionManager {
    regions: HashMap<String, RegionConfig>,
    collectors: HashMap<CollectorId, RegionCollector>,
    processes: HashMap<CollectorId, tokio::process::Child>,
    locations: HashMap<LocationId, LocationConfig>,
    health_status: HashMap<LocationId, crate::types::LocationHealth>,
    storage_path: String,
    default_batch_size: usize,
}

impl RegionManager {
    pub fn new(storage_path: String, default_batch_size: usize) -> Self {
        Self {
            regions: HashMap::new(),
            collectors: HashMap::new(),
            processes: HashMap::new(),
            locations: HashMap::new(),
            health_status: HashMap::new(),
            storage_path,
            default_batch_size,
        }
    }
    
    /// Add a region configuration
    pub fn add_region(&mut self, region_name: String, config: RegionConfig) {
        self.regions.insert(region_name, config);
    }
    
    /// Add a location to a specific region
    pub fn add_location_to_region(&mut self, location_id: LocationId, config: LocationConfig, region_name: String) {
        self.locations.insert(location_id.clone(), config);
        self.health_status.insert(location_id.clone(), crate::types::LocationHealth {
            location_id: location_id.clone(),
            status: crate::types::HealthStatus::Unknown,
            last_data_received: None,
            error_count: 0,
            last_error: None,
        });
        
        // If region doesn't exist, create it with default config
        if !self.regions.contains_key(&region_name) {
            self.regions.insert(region_name.clone(), RegionConfig {
                name: region_name.clone(),
                description: None,
                max_locations: 10,
                batch_size: None,
                priority: 5,
            });
        }
    }
    
    /// Start all collectors based on region assignments
    pub async fn start_all_collectors(&mut self) -> Result<()> {
        info!("Starting region-based collectors for {} locations", self.locations.len());
        
        // Group locations by region
        let region_groups = self.group_locations_by_region();
        info!("Created {} region groups", region_groups.len());
        
        // Start each region collector
        for (region_name, locations) in region_groups {
            self.start_region_collector(region_name, locations).await?;
        }
        
        Ok(())
    }
    
    /// Group locations by their assigned regions
    fn group_locations_by_region(&self) -> HashMap<String, Vec<LocationId>> {
        let mut region_groups: HashMap<String, Vec<LocationId>> = HashMap::new();
        
        // For now, we'll use a simple approach - you can extend this
        // to read region assignments from location metadata
        let mut current_region = "default".to_string();
        let mut region_count = 0;
        
        for location_id in self.locations.keys() {
            // Simple round-robin assignment for demo
            // In practice, you'd read this from location metadata
            if region_count >= 5 { // 5 locations per region
                current_region = format!("region_{}", region_groups.len() + 1);
                region_count = 0;
            }
            
            region_groups.entry(current_region.clone())
                .or_insert_with(Vec::new)
                .push(location_id.clone());
            region_count += 1;
        }
        
        region_groups
    }
    
    /// Start a collector for a specific region
    async fn start_region_collector(&mut self, region_name: String, location_ids: Vec<LocationId>) -> Result<()> {
        info!("Starting collector for region '{}' with {} locations", region_name, location_ids.len());
        
        // Get region configuration
        let region_config = self.regions.get(&region_name)
            .cloned()
            .unwrap_or_else(|| RegionConfig {
                name: region_name.clone(),
                description: None,
                max_locations: 10,
                batch_size: None,
                priority: 5,
            });
        
        let batch_size = region_config.batch_size.unwrap_or(self.default_batch_size);
        
        // Prepare location configurations for this region
        let location_configs: Vec<_> = location_ids.iter()
            .filter_map(|id| self.locations.get(id))
            .map(|config| MultiLocationConfig {
                id: config.id.clone(),
                name: config.name.clone(),
                coordinates: config.coordinates,
                api_key: config.weather_config.api_key.clone(),
                provider: config.weather_config.provider.clone(),
                units: config.weather_config.units.clone(),
                collection_interval: config.weather_config.collection_interval,
            })
            .collect();
        
        // Spawn the multi-collector process
        let collector_id = uuid::Uuid::new_v4();
        let (process_id, child) = self.spawn_region_collector_process(&collector_id, &location_configs, &region_name, batch_size).await?;
        
        // Create region collector handle
        let region_collector = RegionCollector {
            id: collector_id,
            region_name: region_name.clone(),
            locations: location_ids.clone(),
            status: CollectorStatus::Starting,
            last_heartbeat: Instant::now(),
            process_id: Some(process_id),
            batch_size,
        };
        
        // Store collector and process
        self.collectors.insert(collector_id, region_collector);
        self.processes.insert(collector_id, child);
        
        // Update health status for all locations in this region
        for location_id in &location_ids {
            if let Some(health) = self.health_status.get_mut(location_id) {
                health.status = crate::types::HealthStatus::Healthy;
            }
        }
        
        info!("Started region collector '{}' (PID: {}) for {} locations", 
              region_name, process_id, location_ids.len());
        
        Ok(())
    }
    
    /// Spawn a region collector process
    async fn spawn_region_collector_process(
        &self,
        collector_id: &CollectorId,
        location_configs: &[MultiLocationConfig],
        region_name: &str,
        batch_size: usize,
    ) -> Result<(u32, tokio::process::Child)> {
        // Serialize location configurations
        let locations_json = serde_json::to_string(location_configs)
            .context("Failed to serialize location configurations")?;
        
        // Spawn the multi-collector process
        let binary_path = if cfg!(debug_assertions) {
            "/home/autumn/projects/currents/target/debug/multi-collector"
        } else {
            "/home/autumn/projects/currents/target/release/multi-collector"
        };
        
        let mut command = Command::new(binary_path);
        command
            .env("COLLECTOR_ID", collector_id.to_string())
            .env("COLLECTOR_REGION", region_name)
            .env("COLLECTOR_LOCATIONS", locations_json)
            .env("COLLECTOR_STORAGE_PATH", &self.storage_path)
            .env("COLLECTOR_BATCH_SIZE", batch_size.to_string())
            .env("COLLECTOR_MAX_RETRIES", "3")
            .env("COLLECTOR_RETRY_DELAY", "60");

        let child = command.spawn()
            .context("Failed to spawn region collector process")?;

        let process_id = child.id().unwrap_or(0);
        
        Ok((process_id, child))
    }
    
    /// Stop a region collector
    pub async fn stop_region_collector(&mut self, collector_id: &CollectorId) -> Result<()> {
        if let Some(collector) = self.collectors.remove(collector_id) {
            info!("Stopping region collector '{}' for {} locations", 
                  collector.region_name, collector.locations.len());
        }
        
        // Stop and remove process if running
        if let Some(mut process) = self.processes.remove(collector_id) {
            if let Err(e) = process.kill().await {
                warn!("Failed to kill process for collector {}: {}", collector_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Stop all region collectors
    pub async fn stop_all_collectors(&mut self) -> Result<()> {
        info!("Stopping all region collectors");
        
        for collector_id in self.collectors.keys().cloned().collect::<Vec<_>>() {
            self.stop_region_collector(&collector_id).await?;
        }
        
        Ok(())
    }
    
    /// Get collector status
    pub fn get_collector_status(&self, collector_id: &CollectorId) -> Option<&RegionCollector> {
        self.collectors.get(collector_id)
    }
    
    /// Get all collectors
    pub fn get_all_collectors(&self) -> &HashMap<CollectorId, RegionCollector> {
        &self.collectors
    }
    
    /// Get collectors by region
    pub fn get_collectors_by_region(&self, region_name: &str) -> Vec<&RegionCollector> {
        self.collectors.values()
            .filter(|collector| collector.region_name == region_name)
            .collect()
    }
    
    /// Get health status for a location
    pub fn get_location_health(&self, location_id: &LocationId) -> Option<&crate::types::LocationHealth> {
        self.health_status.get(location_id)
    }
    
    /// Get all health statuses
    pub fn get_all_health_statuses(&self) -> &HashMap<LocationId, crate::types::LocationHealth> {
        &self.health_status
    }
    
    /// Restart a region collector
    pub async fn restart_region_collector(&mut self, collector_id: &CollectorId) -> Result<()> {
        info!("Restarting region collector {}", collector_id);
        
        // Get the locations for this collector
        let locations = if let Some(collector) = self.collectors.get(collector_id) {
            collector.locations.clone()
        } else {
            return Err(anyhow::anyhow!("Collector not found: {}", collector_id));
        };
        
        let region_name = if let Some(collector) = self.collectors.get(collector_id) {
            collector.region_name.clone()
        } else {
            return Err(anyhow::anyhow!("Collector not found: {}", collector_id));
        };
        
        // Stop the existing collector
        self.stop_region_collector(collector_id).await?;
        
        // Wait a bit before restarting
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Start a new collector with the same locations
        self.start_region_collector(region_name, locations).await?;
        
        Ok(())
    }
    
    /// Perform health checks on all collectors
    pub async fn perform_health_checks(&mut self) -> Result<()> {
        let mut failed_collectors = Vec::new();
        
        for (collector_id, collector) in &self.collectors {
            // Check if process is still running
            if let Some(process) = self.processes.get_mut(collector_id) {
                if process.try_wait()?.is_some() {
                    warn!("Region collector '{}' process has terminated", collector.region_name);
                    failed_collectors.push(*collector_id);
                }
            } else {
                warn!("Region collector '{}' has no associated process", collector.region_name);
                failed_collectors.push(*collector_id);
            }
        }
        
        // Restart failed collectors
        for collector_id in failed_collectors {
            warn!("Restarting failed region collector {}", collector_id);
            if let Err(e) = self.restart_region_collector(&collector_id).await {
                error!("Failed to restart region collector {}: {}", collector_id, e);
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MultiLocationConfig {
    id: String,
    name: String,
    coordinates: (f64, f64),
    api_key: String,
    provider: String,
    units: String,
    collection_interval: u64,
}
