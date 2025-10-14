//! Currents Orchestrator - Multi-location weather monitoring coordination
//!
//! This crate provides the orchestration layer for managing multiple weather
//! collectors across different locations, enabling cross-location analysis
//! and centralized coordination.

pub mod location_manager;
pub mod collector_coordinator;
pub mod cross_location_analyzer;
pub mod region_manager;
pub mod config;
pub mod unified_config;
pub mod error_handling;
pub mod global_api_tracker;
pub mod types;

pub use location_manager::LocationManager;
pub use collector_coordinator::CollectorCoordinator;
pub use cross_location_analyzer::CrossLocationAnalyzer;
pub use region_manager::RegionManager;
pub use unified_config::{UnifiedConfig, CollectorConfig};
pub use error_handling::{ErrorRecoveryManager, HealthMonitor, RecoveryStrategy, ErrorCategory};
pub use global_api_tracker::GlobalApiTracker;
pub use types::OrchestratorConfig;
pub use types::*;