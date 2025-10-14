//! Core types for the orchestrator system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

/// Unique identifier for a location
pub type LocationId = String;

/// Unique identifier for a collector instance
pub type CollectorId = Uuid;

/// Status of a collector instance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollectorStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error(String),
}

/// Configuration for a specific location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationConfig {
    pub id: LocationId,
    pub name: String,
    pub coordinates: (f64, f64), // (latitude, longitude)
    pub weather_config: WeatherConfig,
    pub collection_strategy: CollectionStrategy,
}

/// Weather configuration for a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    pub api_key: String,
    pub provider: String,
    pub units: String,
    pub collection_interval: u64, // seconds
}

/// Collection strategy for a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectionStrategy {
    Interval,    // Collect at regular intervals
    Threshold,   // Collect when conditions change significantly
    Hybrid,      // Combination of both
}

/// Handle for managing a collector instance
#[derive(Debug, Clone)]
pub struct CollectorHandle {
    pub id: CollectorId,
    pub location_id: LocationId,
    pub status: CollectorStatus,
    pub last_heartbeat: Instant,
    pub process_id: Option<u32>,
}

/// Health status of a location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning(String),
    Critical(String),
    Unknown,
}

/// Health information for a location
#[derive(Debug, Clone)]
pub struct LocationHealth {
    pub location_id: LocationId,
    pub status: HealthStatus,
    pub last_data_received: Option<Instant>,
    pub error_count: u32,
    pub last_error: Option<String>,
}

/// Cross-location correlation data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationCorrelation {
    pub location_a: LocationId,
    pub location_b: LocationId,
    pub correlation_type: CorrelationType,
    pub strength: f64, // 0.0 to 1.0
    pub confidence: f64, // 0.0 to 1.0
}

/// Type of correlation between locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrelationType {
    Temperature,
    Humidity,
    WindSpeed,
    Pressure,
    Precipitation,
}

/// Weather pattern detected across locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPattern {
    pub pattern_id: String,
    pub pattern_type: PatternType,
    pub affected_locations: Vec<LocationId>,
    pub severity: PatternSeverity,
    pub description: String,
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

/// Type of weather pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternType {
    TemperatureGradient,
    PressureSystem,
    WindPattern,
    PrecipitationBand,
    StormSystem,
}

/// Severity of a weather pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternSeverity {
    Low,
    Medium,
    High,
    Extreme,
}

/// Regional analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionalAnalysis {
    pub region: String,
    pub locations: Vec<LocationId>,
    pub patterns: Vec<WeatherPattern>,
    pub correlations: Vec<LocationCorrelation>,
    pub analysis_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Message types for collector communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollectorMessage {
    Heartbeat { collector_id: CollectorId, status: CollectorStatus },
    DataReceived { location_id: LocationId, data_count: u32 },
    Error { collector_id: CollectorId, error: String },
    Shutdown { collector_id: CollectorId },
}

/// Configuration for the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    pub storage_path: String,
    pub analysis_interval: u64, // seconds
    pub health_check_interval: u64, // seconds
    pub batch_size: Option<usize>, // locations per collector process
    pub locations: HashMap<LocationId, LocationConfig>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            storage_path: "~/.config/currents/orchestrator.db".to_string(),
            analysis_interval: 3600, // 1 hour
            health_check_interval: 300, // 5 minutes
            batch_size: Some(5),
            locations: HashMap::new(),
        }
    }
}