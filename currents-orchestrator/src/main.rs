//! Currents Orchestrator - Multi-location weather monitoring coordination

use currents_orchestrator::{
    LocationManager, CollectorCoordinator, CrossLocationAnalyzer,
    OrchestratorConfig
};
use currents_storage::WeatherStorage;
use anyhow::{Result, Context};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, error, debug};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting Currents Orchestrator");

    // Load configuration
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "~/.config/currents/orchestrator.toml".to_string());
    
    let config = OrchestratorConfig::from_file(&config_path)
        .with_context(|| format!("Failed to load configuration from: {}", config_path))?;
    
    // Validate configuration
    config.validate()
        .context("Configuration validation failed")?;

    info!("Loaded configuration with {} locations", config.locations.len());

    // Initialize storage
    let storage_config = currents_storage::types::StorageConfig {
        database_path: config.storage_path.clone(),
        max_history_days: 365,
        enable_compression: true,
        compression_threshold: 30,
    };
    
    let storage = Arc::new(
        WeatherStorage::new(&config.storage_path, storage_config)
            .context("Failed to initialize weather storage")?
    );

    // Create orchestrator components
    let mut location_manager = LocationManager::new(
        config.locations.clone(),
        Duration::from_secs(config.health_check_interval),
    );

    let mut collector_coordinator = CollectorCoordinator::new(
        Duration::from_secs(config.health_check_interval),
    );

    let cross_location_analyzer = CrossLocationAnalyzer::new(storage.clone());

    // Start collectors using multi-location approach (5-10 locations per process)
    match location_manager.start_all_collectors().await {
        Ok(()) => {
            info!("Started multi-location collectors for {} locations", config.locations.len());
        }
        Err(e) => {
            error!("Failed to start multi-location collectors: {}", e);
        }
    }

    // Start the orchestrator main loop
    run_orchestrator(
        location_manager,
        collector_coordinator,
        cross_location_analyzer,
        config,
    ).await?;

    Ok(())
}

async fn run_orchestrator(
    mut location_manager: LocationManager,
    mut collector_coordinator: CollectorCoordinator,
    cross_location_analyzer: CrossLocationAnalyzer,
    config: OrchestratorConfig,
) -> Result<()> {
    info!("Orchestrator main loop started");

    // Start background tasks
    let health_task = tokio::spawn(async move {
        location_manager.start_health_monitoring().await;
    });

    let coordinator_task = tokio::spawn(async move {
        collector_coordinator.start().await
    });

    // Start analysis task (run in main thread for now due to Send constraints)
    let analysis_interval = Duration::from_secs(config.analysis_interval);
    let location_ids: Vec<String> = config.locations.keys().cloned().collect();

    // Run analysis in main loop
    let mut analysis_interval = interval(analysis_interval);
    
    // Wait for any task to complete (or fail)
    tokio::select! {
        result = health_task => {
            if let Err(e) = result {
                error!("Health monitoring task failed: {}", e);
            }
        }
        result = coordinator_task => {
            if let Err(e) = result {
                error!("Collector coordinator task failed: {}", e);
            }
        }
        _ = analysis_interval.tick() => {
            // Perform cross-location analysis
            if location_ids.len() >= 2 {
                match cross_location_analyzer.analyze_region(
                    "global",
                    &location_ids,
                    chrono::Duration::hours(24),
                ).await {
                    Ok(analysis) => {
                        info!("Regional analysis complete: {} patterns, {} correlations",
                              analysis.patterns.len(), analysis.correlations.len());
                        
                        // Log detected patterns
                        for pattern in &analysis.patterns {
                            info!("Detected pattern: {:?} - {}", pattern.pattern_type, pattern.description);
                        }
                    }
                    Err(e) => {
                        warn!("Regional analysis failed: {}", e);
                    }
                }
            }
        }
    }

    info!("Orchestrator main loop ended");
    Ok(())
}