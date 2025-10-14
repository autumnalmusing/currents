//! Currents Orchestrator - Multi-location weather monitoring coordination

use currents_orchestrator::{
    LocationManager, CollectorCoordinator, CrossLocationAnalyzer, RegionManager,
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

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = "~/.config/currents/orchestrator.toml".to_string();
    let mut use_regions = false;
    
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--config" | "-c" => {
                if i + 1 < args.len() {
                    config_path = args[i + 1].clone();
                }
            }
            "--regions" | "-r" => {
                use_regions = true;
            }
            "--help" | "-h" => {
                println!("Currents Orchestrator - Multi-location weather monitoring");
                println!();
                println!("Usage: currents-orchestrator [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -c, --config PATH    Configuration file path");
                println!("  -r, --regions        Use region-based collector grouping");
                println!("  -h, --help           Show this help message");
                println!();
                println!("Examples:");
                println!("  currents-orchestrator                                    # Use default config");
                println!("  currents-orchestrator -c /path/to/config.toml            # Custom config");
                println!("  currents-orchestrator --regions                          # Use region grouping");
                println!("  currents-orchestrator -c config.toml --regions           # Custom config with regions");
                return Ok(());
            }
            _ => {}
        }
    }
    
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

    // Choose collector approach based on command line flag
    if use_regions {
        info!("Using region-based collector grouping");
        
        // Create region manager
        let mut region_manager = RegionManager::new(
            config.storage_path.clone(),
            config.batch_size.unwrap_or(5),
        );
        
        // Add locations to regions (for now, simple round-robin assignment)
        // In practice, you'd read region assignments from config
        let mut region_count = 0;
        let mut current_region = "default".to_string();
        
        for (location_id, location_config) in &config.locations {
            if region_count >= 5 { // 5 locations per region
                current_region = format!("region_{}", region_count / 5 + 1);
            }
            
            region_manager.add_location_to_region(
                location_id.clone(),
                location_config.clone(),
                current_region.clone(),
            );
            region_count += 1;
        }
        
        // Start region-based collectors
        match region_manager.start_all_collectors().await {
            Ok(()) => {
                info!("Started region-based collectors for {} locations", config.locations.len());
            }
            Err(e) => {
                error!("Failed to start region-based collectors: {}", e);
            }
        }
    } else {
        info!("Using automatic batching approach");
        
        // Start collectors using multi-location approach (5-10 locations per process)
        match location_manager.start_all_collectors().await {
            Ok(()) => {
                info!("Started multi-location collectors for {} locations", config.locations.len());
            }
            Err(e) => {
                error!("Failed to start multi-location collectors: {}", e);
            }
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