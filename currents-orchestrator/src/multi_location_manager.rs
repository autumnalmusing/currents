//! Multi-location collector management
//! 
//! Manages collectors that handle multiple locations (5-10 per process)
//! for efficient resource usage in large-scale deployments.

use crate::types::{LocationId, CollectorId, LocationConfig, CollectorHandle, CollectorStatus};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::interval;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLocationCollector {
    pub id: CollectorId,
    pub locations: Vec<LocationId>,
    pub status: CollectorStatus,
    pub last_heartbeat: Instant,
    pub process_id: Option<u32>,
    pub batch_size: usize,
}

/// Manages multi-location collectors for efficient resource usage
pub struct MultiLocationManager {
    collectors: HashMap<CollectorId, MultiLocationCollector>,
    processes: HashMap<CollectorId, tokio::process::Child>,
    locations: HashMap<LocationId, LocationConfig>,
    health_status: HashMap<LocationId, crate::types::LocationHealth>,
    storage_path: String,
    batch_size: usize,
    max_locations_per_collector: usize,
}

impl MultiLocationManager {
    pub fn new(storage_path: String, batch_size: usize) -> Self {
        Self {
            collectors: HashMap::new(),
            processes: HashMap::new(),
            locations: HashMap::new(),
            health_status: HashMap::new(),
            storage_path,
            batch_size,
            max_locations_per_collector: 10, // Max 10 locations per collector
        }
    }
    
    /// Add a location to be managed
    pub fn add_location(&mut self, location_id: LocationId, config: LocationConfig) {
        self.locations.insert(location_id.clone(), config);
        self.health_status.insert(location_id, crate::types::LocationHealth {
            location_id,
            status: crate::types::HealthStatus::Unknown,
            last_check: Instant::now(),
            error_count: 0,
        });
    }
    
    /// Start all collectors based on location distribution
    pub async fn start_all_collectors(&mut self) -> Result<()> {
        info!("Starting multi-location collectors for {} locations", self.locations.len());
        
        // Distribute locations across collectors
        let location_groups = self.distribute_locations();
        info!("Created {} collector groups", location_groups.len());
        
        // Start each collector group
        for (group_id, locations) in location_groups {
            self.start_collector_group(group_id, locations).await?;
        }
        
        Ok(())
    }
    
    /// Distribute locations across collectors
    fn distribute_locations(&self) -> Vec<(CollectorId, Vec<LocationId>)> {
        let mut groups = Vec::new();
        let mut current_group = Vec::new();
        let mut current_collector_id = uuid::Uuid::new_v4();
        
        for location_id in self.locations.keys() {
            current_group.push(location_id.clone());
            
            // If group is full or we've reached the end, create a new group
            if current_group.len() >= self.batch_size {
                groups.push((current_collector_id, current_group));
                current_group = Vec::new();
                current_collector_id = uuid::Uuid::new_v4();
            }
        }
        
        // Add remaining locations
        if !current_group.is_empty() {
            groups.push((current_collector_id, current_group));
        }
        
        groups
    }
    
    /// Start a collector group
    async fn start_collector_group(&mut self, collector_id: CollectorId, location_ids: Vec<LocationId>) -> Result<()> {
        info!("Starting collector {} for {} locations", collector_id, location_ids.len());
        
        // Prepare location configurations for this collector
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
        let (process_id, child) = self.spawn_multi_collector_process(&collector_id, &location_configs).await?;
        
        // Create collector handle
        let collector = MultiLocationCollector {
            id: collector_id,
            locations: location_ids.clone(),
            status: CollectorStatus::Starting,
            last_heartbeat: Instant::now(),
            process_id: Some(process_id),
            batch_size: self.batch_size,
        };
        
        // Store collector and process
        self.collectors.insert(collector_id, collector);
        self.processes.insert(collector_id, child);
        
        // Update health status for all locations in this collector
        for location_id in &location_ids {
            if let Some(health) = self.health_status.get_mut(location_id) {
                health.status = crate::types::HealthStatus::Healthy;
            }
        }
        
        info!("Started multi-location collector {} (PID: {}) for {} locations", 
              collector_id, process_id, location_ids.len());
        
        Ok(())
    }
    
    /// Spawn a multi-collector process
    async fn spawn_multi_collector_process(
        &self,
        collector_id: &CollectorId,
        location_configs: &[MultiLocationConfig],
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
            .env("COLLECTOR_LOCATIONS", locations_json)
            .env("COLLECTOR_STORAGE_PATH", &self.storage_path)
            .env("COLLECTOR_BATCH_SIZE", self.batch_size.to_string())
            .env("COLLECTOR_MAX_RETRIES", "3")
            .env("COLLECTOR_RETRY_DELAY", "60");

        let child = command.spawn()
            .context("Failed to spawn multi-collector process")?;

        let process_id = child.id().unwrap_or(0);
        
        Ok((process_id, child))
    }
    
    /// Stop a collector
    pub async fn stop_collector(&mut self, collector_id: &CollectorId) -> Result<()> {
        if let Some(collector) = self.collectors.remove(collector_id) {
            info!("Stopping multi-location collector {} for {} locations", 
                  collector.id, collector.locations.len());
        }
        
        // Stop and remove process if running
        if let Some(mut process) = self.processes.remove(collector_id) {
            if let Err(e) = process.kill().await {
                warn!("Failed to kill process for collector {}: {}", collector_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Stop all collectors
    pub async fn stop_all_collectors(&mut self) -> Result<()> {
        info!("Stopping all multi-location collectors");
        
        for collector_id in self.collectors.keys().cloned().collect::<Vec<_>>() {
            self.stop_collector(&collector_id).await?;
        }
        
        Ok(())
    }
    
    /// Get collector status
    pub fn get_collector_status(&self, collector_id: &CollectorId) -> Option<&MultiLocationCollector> {
        self.collectors.get(collector_id)
    }
    
    /// Get all collectors
    pub fn get_all_collectors(&self) -> &HashMap<CollectorId, MultiLocationCollector> {
        &self.collectors
    }
    
    /// Get health status for a location
    pub fn get_location_health(&self, location_id: &LocationId) -> Option<&crate::types::LocationHealth> {
        self.health_status.get(location_id)
    }
    
    /// Get all health statuses
    pub fn get_all_health_statuses(&self) -> &HashMap<LocationId, crate::types::LocationHealth> {
        &self.health_status
    }
    
    /// Restart a collector
    pub async fn restart_collector(&mut self, collector_id: &CollectorId) -> Result<()> {
        info!("Restarting multi-location collector {}", collector_id);
        
        // Get the locations for this collector
        let locations = if let Some(collector) = self.collectors.get(collector_id) {
            collector.locations.clone()
        } else {
            return Err(anyhow::anyhow!("Collector not found: {}", collector_id));
        };
        
        // Stop the existing collector
        self.stop_collector(collector_id).await?;
        
        // Wait a bit before restarting
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Start a new collector with the same locations
        self.start_collector_group(*collector_id, locations).await?;
        
        Ok(())
    }
    
    /// Perform health checks on all collectors
    pub async fn perform_health_checks(&mut self) -> Result<()> {
        let mut failed_collectors = Vec::new();
        
        for (collector_id, collector) in &self.collectors {
            // Check if process is still running
            if let Some(process) = self.processes.get(collector_id) {
                if process.try_wait()?.is_some() {
                    warn!("Collector {} process has terminated", collector_id);
                    failed_collectors.push(*collector_id);
                }
            } else {
                warn!("Collector {} has no associated process", collector_id);
                failed_collectors.push(*collector_id);
            }
        }
        
        // Restart failed collectors
        for collector_id in failed_collectors {
            warn!("Restarting failed collector {}", collector_id);
            if let Err(e) = self.restart_collector(&collector_id).await {
                error!("Failed to restart collector {}: {}", collector_id, e);
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
