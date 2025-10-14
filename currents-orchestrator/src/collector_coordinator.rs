//! Collector coordination and lifecycle management

use crate::types::{
    CollectorId, CollectorHandle, CollectorStatus, CollectorMessage, LocationId
};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::interval;
use tokio::sync::mpsc;
use tokio::process::Command;
use tracing::{info, warn, error, debug};

/// Manages the lifecycle of multiple collector instances
pub struct CollectorCoordinator {
    collectors: HashMap<LocationId, CollectorHandle>,
    processes: HashMap<LocationId, tokio::process::Child>,
    message_sender: mpsc::UnboundedSender<CollectorMessage>,
    message_receiver: mpsc::UnboundedReceiver<CollectorMessage>,
    health_check_interval: Duration,
    max_restart_attempts: u32,
    restart_delay: Duration,
}

impl CollectorCoordinator {
    /// Create a new collector coordinator
    pub fn new(health_check_interval: Duration) -> Self {
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        
        Self {
            collectors: HashMap::new(),
            processes: HashMap::new(),
            message_sender,
            message_receiver,
            health_check_interval,
            max_restart_attempts: 3,
            restart_delay: Duration::from_secs(30),
        }
    }

    /// Add a collector to be managed
    pub fn add_collector(&mut self, collector: CollectorHandle) {
        let location_id = collector.location_id.clone();
        info!("Adding collector {} for location {}", collector.id, location_id);
        self.collectors.insert(location_id, collector);
    }

    /// Remove a collector from management
    pub fn remove_collector(&mut self, location_id: &LocationId) -> Option<CollectorHandle> {
        info!("Removing collector for location {}", location_id);
        self.collectors.remove(location_id)
    }

    /// Get all managed collectors
    pub fn get_collectors(&self) -> &HashMap<LocationId, CollectorHandle> {
        &self.collectors
    }

    /// Get a specific collector
    pub fn get_collector(&self, location_id: &LocationId) -> Option<&CollectorHandle> {
        self.collectors.get(location_id)
    }

    /// Start a collector
    pub async fn start_collector(&mut self, location_id: &LocationId) -> Result<CollectorId> {
        // Check if collector is already running
        if self.collectors.contains_key(location_id) {
            return Err(anyhow::anyhow!("Collector already running for location: {}", location_id));
        }

        let collector_id = uuid::Uuid::new_v4();
        
        // Spawn the actual collector process
        let (process_id, child) = self.spawn_collector_process(location_id, &collector_id).await?;
        
        let collector_handle = CollectorHandle {
            id: collector_id,
            location_id: location_id.clone(),
            status: CollectorStatus::Starting,
            last_heartbeat: Instant::now(),
            process_id: Some(process_id),
        };

        self.collectors.insert(location_id.clone(), collector_handle);
        self.processes.insert(location_id.clone(), child);
        
        info!("Started collector {} (PID: {}) for location: {}", collector_id, process_id, location_id);
        Ok(collector_id)
    }

    /// Stop a collector
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
        
        // If we have a process ID, log it for reference
        if let Some(process_id) = collector.process_id {
            info!("Collector {} (PID: {}) will be terminated", collector.id, process_id);
        }
        
        Ok(())
    }

    /// Spawn a collector process
    async fn spawn_collector_process(
        &self,
        location_id: &LocationId,
        collector_id: &CollectorId,
    ) -> Result<(u32, tokio::process::Child)> {
        // Spawn the simple-collector process
        // Use debug binary for tests, release for production
        let binary_path = if cfg!(debug_assertions) {
            "/home/autumn/projects/currents/target/debug/simple-collector"
        } else {
            "/home/autumn/projects/currents/target/release/simple-collector"
        };
        
        let mut command = Command::new(binary_path);
        command
            .env("COLLECTOR_ID", collector_id.to_string())
            .env("COLLECTOR_LOCATION_ID", location_id)
            .env("COLLECTOR_STORAGE_PATH", "~/.config/currents/orchestrator.db");

        let child = command.spawn()
            .context("Failed to spawn collector process")?;

        let process_id = child.id().unwrap_or(0);
        
        Ok((process_id, child))
    }

    /// Restart a collector
    pub async fn restart_collector(&mut self, location_id: &LocationId) -> Result<()> {
        info!("Restarting collector for location: {}", location_id);
        
        // Stop the existing collector
        self.stop_collector(location_id).await?;
        
        // Wait a bit before restarting
        tokio::time::sleep(self.restart_delay).await;
        
        // Start a new collector
        self.start_collector(location_id).await?;
        
        Ok(())
    }

    /// Send a message to all collectors
    pub fn broadcast_message(&self, message: CollectorMessage) -> Result<()> {
        self.message_sender.send(message)
            .context("Failed to send message to collectors")?;
        Ok(())
    }

    /// Send a message to a specific collector
    pub fn send_message_to_collector(&self, collector_id: &CollectorId, message: CollectorMessage) -> Result<()> {
        // TODO: In a real implementation, this would send to a specific collector
        // For now, we'll just log it
        debug!("Sending message to collector {}: {:?}", collector_id, message);
        Ok(())
    }

    /// Start the coordinator's main loop
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting collector coordinator");
        
        // Start health monitoring
        let health_interval = self.health_check_interval;
        let collectors = self.collectors.clone();
        let health_task = tokio::spawn(async move {
            Self::run_health_monitoring(health_interval, collectors).await
        });
        
        // Start message processing
        let mut message_receiver = std::mem::replace(&mut self.message_receiver, mpsc::unbounded_channel().1);
        let message_task = tokio::spawn(async move {
            Self::run_message_processing(&mut message_receiver).await
        });
        
        // Run both tasks concurrently
        tokio::select! {
            result = health_task => {
                if let Err(e) = result {
                    error!("Health monitoring task failed: {}", e);
                }
            }
            result = message_task => {
                if let Err(e) = result {
                    error!("Message processing task failed: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// Run health monitoring task
    async fn run_health_monitoring(
        health_check_interval: Duration,
        collectors: HashMap<LocationId, CollectorHandle>
    ) -> Result<()> {
        let mut interval = interval(health_check_interval);
        let mut restart_counts: HashMap<LocationId, u32> = HashMap::new();
        
        loop {
            interval.tick().await;
            Self::perform_health_checks(&collectors, &mut restart_counts).await;
        }
    }

    /// Perform health checks on all collectors
    async fn perform_health_checks(
        collectors: &HashMap<LocationId, CollectorHandle>,
        restart_counts: &mut HashMap<LocationId, u32>
    ) {
        debug!("Performing health checks on {} collectors", collectors.len());

        for (location_id, collector) in collectors {
            let time_since_heartbeat = Instant::now().duration_since(collector.last_heartbeat);
            
            if time_since_heartbeat > Duration::from_secs(300) { // 5 minutes
                warn!("Collector for location {} has not sent heartbeat for {} seconds", 
                      location_id, time_since_heartbeat.as_secs());
                
                // Check if we should restart
                let restart_count = restart_counts.get(location_id).unwrap_or(&0);
                if *restart_count < 3 { // max_restart_attempts
                    warn!("Attempting to restart collector for location {} (attempt {})", 
                          location_id, restart_count + 1);
                    
                    // In a real implementation, this would restart the collector
                    // For now, we'll just increment the restart count
                    restart_counts.insert(location_id.clone(), restart_count + 1);
                } else {
                    error!("Max restart attempts reached for collector at location {}", location_id);
                }
            }
        }
    }

    /// Run message processing task
    async fn run_message_processing(
        message_receiver: &mut mpsc::UnboundedReceiver<CollectorMessage>
    ) -> Result<()> {
        while let Some(message) = message_receiver.recv().await {
            debug!("Received message: {:?}", message);
            
            // Handle different message types
            match message {
                CollectorMessage::Heartbeat { collector_id, status } => {
                    debug!("Heartbeat from collector {}: {:?}", collector_id, status);
                }
                CollectorMessage::DataReceived { location_id, data_count } => {
                    info!("Location {} received {} data points", location_id, data_count);
                }
                CollectorMessage::Error { collector_id, error } => {
                    error!("Collector {} reported error: {}", collector_id, error);
                }
                CollectorMessage::Shutdown { collector_id } => {
                    info!("Collector {} requested shutdown", collector_id);
                }
            }
        }
        Ok(())
    }

    /// Handle incoming messages from collectors
    async fn handle_message(&mut self, message: CollectorMessage) -> Result<()> {
        match message {
            CollectorMessage::Heartbeat { collector_id, status } => {
                self.handle_heartbeat(collector_id, status).await?;
            }
            CollectorMessage::DataReceived { location_id, data_count } => {
                self.handle_data_received(location_id, data_count).await?;
            }
            CollectorMessage::Error { collector_id, error } => {
                self.handle_error(collector_id, error).await?;
            }
            CollectorMessage::Shutdown { collector_id } => {
                self.handle_shutdown(collector_id).await?;
            }
        }
        Ok(())
    }

    /// Handle heartbeat from a collector
    async fn handle_heartbeat(&mut self, collector_id: CollectorId, status: CollectorStatus) -> Result<()> {
        for collector in self.collectors.values_mut() {
            if collector.id == collector_id {
                collector.last_heartbeat = Instant::now();
                collector.status = status.clone();
                debug!("Received heartbeat from collector {}: {:?}", collector_id, status);
                break;
            }
        }
        Ok(())
    }

    /// Handle data received notification
    async fn handle_data_received(&self, location_id: LocationId, data_count: u32) -> Result<()> {
        debug!("Location {} received {} data points", location_id, data_count);
        Ok(())
    }

    /// Handle error from a collector
    async fn handle_error(&mut self, collector_id: CollectorId, error: String) -> Result<()> {
        error!("Collector {} reported error: {}", collector_id, error);
        
        // Find the collector and update its status
        for collector in self.collectors.values_mut() {
            if collector.id == collector_id {
                collector.status = CollectorStatus::Error(error.clone());
                break;
            }
        }
        
        Ok(())
    }

    /// Handle shutdown request from a collector
    async fn handle_shutdown(&mut self, collector_id: CollectorId) -> Result<()> {
        info!("Collector {} requested shutdown", collector_id);
        
        // Find and remove the collector
        let location_id = self.collectors
            .iter()
            .find(|(_, collector)| collector.id == collector_id)
            .map(|(location_id, _)| location_id.clone());
        
        if let Some(location_id) = location_id {
            self.collectors.remove(&location_id);
        }
        
        Ok(())
    }

    /// Get statistics about the coordinator
    pub fn get_statistics(&self) -> CollectorCoordinatorStats {
        let total_collectors = self.collectors.len();
        let running_collectors = self.collectors
            .values()
            .filter(|collector| matches!(collector.status, CollectorStatus::Running))
            .count();
        let starting_collectors = self.collectors
            .values()
            .filter(|collector| matches!(collector.status, CollectorStatus::Starting))
            .count();
        let error_collectors = self.collectors
            .values()
            .filter(|collector| matches!(collector.status, CollectorStatus::Error(_)))
            .count();

        CollectorCoordinatorStats {
            total_collectors,
            running_collectors,
            starting_collectors,
            error_collectors,
        }
    }

    /// Shutdown all collectors gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down collector coordinator");
        
        // Send shutdown message to all collectors
        let shutdown_message = CollectorMessage::Shutdown { 
            collector_id: uuid::Uuid::new_v4() // This would be handled differently in real implementation
        };
        self.broadcast_message(shutdown_message)?;
        
        // Stop all collectors
        let location_ids: Vec<LocationId> = self.collectors.keys().cloned().collect();
        for location_id in location_ids {
            self.stop_collector(&location_id).await?;
        }
        
        info!("Collector coordinator shutdown complete");
        Ok(())
    }
}

/// Statistics about the collector coordinator
#[derive(Debug, Clone)]
pub struct CollectorCoordinatorStats {
    pub total_collectors: usize,
    pub running_collectors: usize,
    pub starting_collectors: usize,
    pub error_collectors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_add_collector() {
        let mut coordinator = CollectorCoordinator::new(Duration::from_secs(60));
        
        let collector = CollectorHandle {
            id: uuid::Uuid::new_v4(),
            location_id: "london".to_string(),
            status: CollectorStatus::Running,
            last_heartbeat: Instant::now(),
            process_id: Some(12345),
        };
        
        coordinator.add_collector(collector);
        assert_eq!(coordinator.get_collectors().len(), 1);
    }

    #[tokio::test]
    async fn test_start_collector() {
        let mut coordinator = CollectorCoordinator::new(Duration::from_secs(60));
        
        let collector_id = coordinator.start_collector(&"london".to_string()).await.unwrap();
        assert_eq!(coordinator.get_collectors().len(), 1);
        assert!(coordinator.get_collector(&"london".to_string()).is_some());
    }

    #[tokio::test]
    async fn test_handle_heartbeat() {
        let mut coordinator = CollectorCoordinator::new(Duration::from_secs(60));
        
        let collector_id = coordinator.start_collector(&"london".to_string()).await.unwrap();
        
        let message = CollectorMessage::Heartbeat {
            collector_id,
            status: CollectorStatus::Running,
        };
        
        coordinator.handle_message(message).await.unwrap();
        
        let collector = coordinator.get_collector(&"london".to_string()).unwrap();
        assert_eq!(collector.status, CollectorStatus::Running);
    }
}