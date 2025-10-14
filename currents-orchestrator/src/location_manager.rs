//! Location management system for the orchestrator

use crate::types::{
    LocationConfig, LocationId, LocationHealth, HealthStatus, CollectorHandle, CollectorId
};
use crate::error_handling::{ErrorRecoveryManager, HealthMonitor, RecoveryStrategy};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tokio::process::Command;
use tracing::{info, warn, error, debug};
use serde::{Deserialize, Serialize};

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

/// Manages multiple weather collection locations
pub struct LocationManager {
    locations: HashMap<LocationId, LocationConfig>,
    collectors: HashMap<LocationId, CollectorHandle>,
    processes: HashMap<LocationId, tokio::process::Child>,
    health_status: HashMap<LocationId, LocationHealth>,
    health_check_interval: Duration,
    error_recovery: ErrorRecoveryManager,
    health_monitor: HealthMonitor,
}

impl LocationManager {
    /// Create a new location manager
    pub fn new(locations: HashMap<LocationId, LocationConfig>, health_check_interval: Duration) -> Self {
        let mut error_recovery = ErrorRecoveryManager::new();
        
        // Configure error recovery strategies
        error_recovery.add_strategy("start_collector", RecoveryStrategy::ExponentialBackoff {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
            max_attempts: 5,
        });
        
        error_recovery.add_strategy("stop_collector", RecoveryStrategy::FixedDelay {
            delay: Duration::from_secs(2),
            max_attempts: 3,
        });
        
        error_recovery.add_strategy("health_check", RecoveryStrategy::FixedDelay {
            delay: Duration::from_secs(1),
            max_attempts: 2,
        });
        
        // Add circuit breakers for critical services
        error_recovery.add_circuit_breaker("weather_api", 5, Duration::from_secs(60));
        error_recovery.add_circuit_breaker("database", 3, Duration::from_secs(30));
        
        let health_monitor = HealthMonitor::new(health_check_interval);
        
        let mut manager = Self {
            locations,
            collectors: HashMap::new(),
            processes: HashMap::new(),
            health_status: HashMap::new(),
            health_check_interval,
            error_recovery,
            health_monitor,
        };

        // Initialize health status for all locations
        for location_id in manager.locations.keys() {
            manager.health_status.insert(
                location_id.clone(),
                LocationHealth {
                    location_id: location_id.clone(),
                    status: HealthStatus::Unknown,
                    last_data_received: None,
                    error_count: 0,
                    last_error: None,
                },
            );
        }

        manager
    }

    /// Get all configured locations
    pub fn get_locations(&self) -> &HashMap<LocationId, LocationConfig> {
        &self.locations
    }

    /// Get a specific location configuration
    pub fn get_location(&self, location_id: &LocationId) -> Option<&LocationConfig> {
        self.locations.get(location_id)
    }

    /// Add a new location
    pub fn add_location(&mut self, location: LocationConfig) -> Result<()> {
        let location_id = location.id.clone();
        
        // Validate location configuration
        self.validate_location(&location)?;
        
        self.locations.insert(location_id.clone(), location);
        
        // Initialize health status for the new location
        self.health_status.insert(
            location_id.clone(),
            LocationHealth {
                location_id: location_id.clone(),
                status: HealthStatus::Unknown,
                last_data_received: None,
                error_count: 0,
                last_error: None,
            },
        );

        info!("Added new location: {}", location_id);
        Ok(())
    }

    /// Remove a location
    pub async fn remove_location(&mut self, location_id: &LocationId) -> Result<()> {
        // Stop collector if running
        if let Some(collector) = self.collectors.remove(location_id) {
            self.stop_collector_handle(&collector).await?;
        }

        // Stop and remove process if running
        if let Some(mut process) = self.processes.remove(location_id) {
            if let Err(e) = process.kill().await {
                warn!("Failed to kill process for location {}: {}", location_id, e);
            }
        }

        self.locations.remove(location_id);
        self.health_status.remove(location_id);

        info!("Removed location: {}", location_id);
        Ok(())
    }

    /// Start collectors for all locations using multi-location approach
    pub async fn start_all_collectors(&mut self) -> Result<()> {
        info!("Starting multi-location collectors for {} locations", self.locations.len());
        
        // Distribute locations across collector groups (5-10 locations per group)
        let location_groups = self.distribute_locations(5); // 5 locations per collector
        info!("Created {} collector groups", location_groups.len());
        
        // Start each collector group
        for (group_id, locations) in location_groups {
            self.start_collector_group(group_id, locations).await?;
        }
        
        Ok(())
    }
    
    /// Distribute locations across collector groups
    fn distribute_locations(&self, batch_size: usize) -> Vec<(CollectorId, Vec<LocationId>)> {
        let mut groups = Vec::new();
        let mut current_group = Vec::new();
        let mut current_collector_id = uuid::Uuid::new_v4();
        
        for location_id in self.locations.keys() {
            current_group.push(location_id.clone());
            
            // If group is full, create a new group
            if current_group.len() >= batch_size {
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
    
    /// Start a collector group (multiple locations per process)
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
        
        // Create collector handle for the group
        let collector_handle = CollectorHandle {
            id: collector_id,
            location_id: format!("group_{}", collector_id), // Use group ID as location ID
            status: crate::types::CollectorStatus::Starting,
            last_heartbeat: Instant::now(),
            process_id: Some(process_id),
        };
        
        // Store collector and process
        self.collectors.insert(format!("group_{}", collector_id), collector_handle);
        self.processes.insert(format!("group_{}", collector_id), child);
        
        // Update health status for all locations in this collector
        for location_id in &location_ids {
            if let Some(health) = self.health_status.get_mut(location_id) {
                health.status = HealthStatus::Healthy;
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
            .env("COLLECTOR_STORAGE_PATH", &self.get_storage_path())
            .env("COLLECTOR_BATCH_SIZE", "5")
            .env("COLLECTOR_MAX_RETRIES", "3")
            .env("COLLECTOR_RETRY_DELAY", "60");

        let child = command.spawn()
            .context("Failed to spawn multi-collector process")?;

        let process_id = child.id().unwrap_or(0);
        
        Ok((process_id, child))
    }

    /// Start a collector for a specific location (legacy method - now redirects to multi-location)
    pub async fn start_collector(&mut self, _location_id: &LocationId) -> Result<CollectorId> {
        // For backward compatibility, start all collectors
        self.start_all_collectors().await?;
        Ok(uuid::Uuid::new_v4())
    }
    
    /// Spawn collector process (internal method)
    fn spawn_collector_process(
        &self,
        location_id: &LocationId,
        location: &LocationConfig,
    ) -> Result<(CollectorId, u32, tokio::process::Child)> {
        let collector_id = uuid::Uuid::new_v4();
        
        // Spawn the simple-collector process
        // Use debug binary for tests, release for production
        let binary_path = if cfg!(debug_assertions) {
            "/home/autumn/projects/currents/target/debug/simple-collector"
        } else {
            "/home/autumn/projects/currents/target/release/simple-collector"
        };
        
        let mut command = Command::new(binary_path);
        command
            .env("COLLECTOR_LOCATION_ID", location_id)
            .env("COLLECTOR_LOCATION_NAME", &location.name)
            .env("COLLECTOR_API_KEY", &location.weather_config.api_key)
            .env("COLLECTOR_PROVIDER", &location.weather_config.provider)
            .env("COLLECTOR_UNITS", &location.weather_config.units)
            .env("COLLECTOR_INTERVAL", location.weather_config.collection_interval.to_string())
            .env("COLLECTOR_STORAGE_PATH", &self.get_storage_path());

        let child = command.spawn()
            .context("Failed to spawn collector process")?;

        let process_id = child.id().unwrap_or(0); // Handle case where process ID might not be available
        
        Ok((collector_id, process_id, child))
    }

    /// Stop a collector for a specific location
    pub async fn stop_collector(&mut self, location_id: &LocationId) -> Result<()> {
        if let Some(collector) = self.collectors.remove(location_id) {
            self.stop_collector_handle(&collector).await?;
        }

        // Stop and remove process if running
        if let Some(mut process) = self.processes.remove(location_id) {
            if let Err(e) = process.kill().await {
                warn!("Failed to kill process for location {}: {}", location_id, e);
            }
        }
        Ok(())
    }

    /// Stop a specific collector handle
    async fn stop_collector_handle(&self, collector: &CollectorHandle) -> Result<()> {
        info!("Stopping collector {} for location: {}", collector.id, collector.location_id);
        
        // If we have a process ID, try to terminate the process
        if let Some(process_id) = collector.process_id {
            // Note: We can't directly kill by PID from here since we don't have the Child handle
            // The actual process termination is handled in stop_collector method
            info!("Collector {} (PID: {}) will be terminated", collector.id, process_id);
        }
        
        Ok(())
    }

    /// Get all active collectors
    pub fn get_collectors(&self) -> &HashMap<LocationId, CollectorHandle> {
        &self.collectors
    }

    /// Get collector for a specific location
    pub fn get_collector(&self, location_id: &LocationId) -> Option<&CollectorHandle> {
        self.collectors.get(location_id)
    }

    /// Update collector heartbeat
    pub fn update_heartbeat(&mut self, collector_id: &CollectorId, status: crate::types::CollectorStatus) {
        for collector in self.collectors.values_mut() {
            if collector.id == *collector_id {
                collector.last_heartbeat = Instant::now();
                collector.status = status;
                break;
            }
        }
    }

    /// Get health status for all locations
    pub fn get_health_status(&self) -> &HashMap<LocationId, LocationHealth> {
        &self.health_status
    }

    /// Get health status for a specific location
    pub fn get_location_health(&self, location_id: &LocationId) -> Option<&LocationHealth> {
        self.health_status.get(location_id)
    }

    /// Update health status for a location
    pub fn update_health_status(&mut self, location_id: &LocationId, health: LocationHealth) {
        self.health_status.insert(location_id.clone(), health);
    }

    /// Start health monitoring
    pub async fn start_health_monitoring(&mut self) {
        let mut interval = interval(self.health_check_interval);
        
        loop {
            interval.tick().await;
            self.perform_health_checks().await;
        }
    }

    /// Perform health checks on all locations
    async fn perform_health_checks(&mut self) {
        debug!("Performing health checks on {} locations", self.locations.len());

        // Check if any processes have exited
        let mut dead_processes = Vec::new();
        for (location_id, process) in self.processes.iter_mut() {
            if let Ok(Some(_)) = process.try_wait() {
                dead_processes.push(location_id.clone());
            }
        }

        // Remove dead processes and their collectors
        for location_id in dead_processes {
            warn!("Process for location {} has exited", location_id);
            self.collectors.remove(&location_id);
            self.processes.remove(&location_id);
            
            if let Some(health) = self.health_status.get_mut(&location_id) {
                health.status = HealthStatus::Critical("Process exited".to_string());
            }
        }

        for (location_id, health) in self.health_status.iter_mut() {
            let collector = self.collectors.get(location_id);
            
            match collector {
                Some(collector) => {
                    let time_since_heartbeat = Instant::now().duration_since(collector.last_heartbeat);
                    
                    if time_since_heartbeat > Duration::from_secs(300) { // 5 minutes
                        health.status = HealthStatus::Critical(
                            format!("No heartbeat for {} seconds", time_since_heartbeat.as_secs())
                        );
                        health.error_count += 1;
                        warn!("Collector for location {} has not sent heartbeat for {} seconds", 
                              location_id, time_since_heartbeat.as_secs());
                    } else if time_since_heartbeat > Duration::from_secs(60) { // 1 minute
                        health.status = HealthStatus::Warning(
                            format!("Heartbeat delayed by {} seconds", time_since_heartbeat.as_secs())
                        );
                    } else {
                        health.status = HealthStatus::Healthy;
                    }
                }
                None => {
                    health.status = HealthStatus::Critical("No collector running".to_string());
                    warn!("No collector running for location: {}", location_id);
                }
            }
        }
    }

    /// Validate a location configuration
    fn validate_location(&self, location: &LocationConfig) -> Result<()> {
        // Check coordinates
        if location.coordinates.0 < -90.0 || location.coordinates.0 > 90.0 {
            return Err(anyhow::anyhow!(
                "Invalid latitude for location '{}': {}",
                location.id,
                location.coordinates.0
            ));
        }

        if location.coordinates.1 < -180.0 || location.coordinates.1 > 180.0 {
            return Err(anyhow::anyhow!(
                "Invalid longitude for location '{}': {}",
                location.id,
                location.coordinates.1
            ));
        }

        // Check API key
        if location.weather_config.api_key.is_empty() {
            return Err(anyhow::anyhow!(
                "API key is required for location '{}'",
                location.id
            ));
        }

        // Check collection interval
        if location.weather_config.collection_interval == 0 {
            return Err(anyhow::anyhow!(
                "Collection interval must be greater than 0 for location '{}'",
                location.id
            ));
        }

        Ok(())
    }

    /// Get locations that need attention (errors, warnings)
    pub fn get_locations_needing_attention(&self) -> Vec<&LocationHealth> {
        self.health_status
            .values()
            .filter(|health| matches!(health.status, HealthStatus::Warning(_) | HealthStatus::Critical(_)))
            .collect()
    }

    /// Get statistics about the location manager
    pub fn get_statistics(&self) -> LocationManagerStats {
        let total_locations = self.locations.len();
        let active_collectors = self.collectors.len();
        let healthy_locations = self.health_status
            .values()
            .filter(|health| matches!(health.status, HealthStatus::Healthy))
            .count();
        let warning_locations = self.health_status
            .values()
            .filter(|health| matches!(health.status, HealthStatus::Warning(_)))
            .count();
        let critical_locations = self.health_status
            .values()
            .filter(|health| matches!(health.status, HealthStatus::Critical(_)))
            .count();

        LocationManagerStats {
            total_locations,
            active_collectors,
            healthy_locations,
            warning_locations,
            critical_locations,
        }
    }

    /// Get process information for all running collectors
    pub fn get_process_info(&self) -> HashMap<LocationId, ProcessInfo> {
        let mut process_info = HashMap::new();
        
        for (location_id, collector) in &self.collectors {
            if let Some(process_id) = collector.process_id {
                process_info.insert(location_id.clone(), ProcessInfo {
                    process_id,
                    status: collector.status.clone(),
                    last_heartbeat: collector.last_heartbeat,
                });
            }
        }
        
        process_info
    }

    /// Get error recovery statistics
    pub fn get_error_stats(&self) -> crate::error_handling::ErrorStats {
        self.error_recovery.get_error_stats()
    }
    
    /// Get health monitoring information
    pub fn get_health_monitoring(&self) -> &HashMap<String, crate::error_handling::HealthCheckResult> {
        self.health_monitor.get_all_health()
    }
    
    /// Cleanup old error contexts
    pub fn cleanup_old_errors(&mut self, max_age: Duration) {
        self.error_recovery.cleanup_old_contexts(max_age);
    }

    /// Get the configured storage path
    fn get_storage_path(&self) -> String {
        // In a real implementation, this would come from configuration
        // For now, we'll use a default path
        "~/.config/currents/orchestrator.db".to_string()
    }
    
    /// Shutdown all collectors and cleanup resources
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down location manager with {} active collectors", self.collectors.len());
        
        // Stop all collectors
        let location_ids: Vec<LocationId> = self.collectors.keys().cloned().collect();
        for location_id in location_ids {
            if let Err(e) = self.stop_collector(&location_id).await {
                error!("Failed to stop collector for location {}: {}", location_id, e);
            }
        }
        
        // Wait a bit for processes to terminate gracefully
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Force kill any remaining processes
        for (location_id, mut process) in self.processes.drain() {
            if let Err(e) = process.kill().await {
                warn!("Failed to force kill process for location {}: {}", location_id, e);
            }
        }
        
        info!("Location manager shutdown complete");
        Ok(())
    }
}

/// Statistics about the location manager
#[derive(Debug, Clone)]
pub struct LocationManagerStats {
    pub total_locations: usize,
    pub active_collectors: usize,
    pub healthy_locations: usize,
    pub warning_locations: usize,
    pub critical_locations: usize,
}

/// Information about a running process
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub process_id: u32,
    pub status: crate::types::CollectorStatus,
    pub last_heartbeat: Instant,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{WeatherConfig, CollectionStrategy};
    use std::collections::HashMap;

    fn create_test_location(id: &str) -> LocationConfig {
        LocationConfig {
            id: id.to_string(),
            name: format!("Test Location {}", id),
            coordinates: (51.5074, -0.1278),
            weather_config: WeatherConfig {
                api_key: "test-key".to_string(),
                provider: "openweathermap".to_string(),
                units: "metric".to_string(),
                collection_interval: 1800,
            },
            collection_strategy: CollectionStrategy::Interval,
        }
    }

    #[tokio::test]
    async fn test_add_location() {
        let mut manager = LocationManager::new(HashMap::new(), Duration::from_secs(60));
        
        let location = create_test_location("london");
        assert!(manager.add_location(location).is_ok());
        assert_eq!(manager.get_locations().len(), 1);
    }

    #[tokio::test]
    async fn test_invalid_location() {
        let mut manager = LocationManager::new(HashMap::new(), Duration::from_secs(60));
        
        let mut location = create_test_location("invalid");
        location.coordinates = (91.0, 0.0); // Invalid latitude
        
        assert!(manager.add_location(location).is_err());
    }

    #[tokio::test]
    async fn test_start_collector() {
        let mut locations = HashMap::new();
        let location = create_test_location("london");
        locations.insert("london".to_string(), location);
        
        let mut manager = LocationManager::new(locations, Duration::from_secs(60));
        
        let collector_id = manager.start_collector(&"london".to_string()).await.unwrap();
        assert_eq!(manager.get_collectors().len(), 1);
        assert!(manager.get_collector(&"london".to_string()).is_some());
    }
}