pub mod analysis;
pub mod config;
pub mod cli;
pub mod visualization;
pub mod collection;

pub use currents_storage::{WeatherStorage, StorageConfig, DatabaseStats, DailyWeather};
pub use analysis::{PatternAnalyzer, TrendData, WeatherPattern};
pub use config::{HistoryConfig, ThresholdSettings};
pub use collection::CollectionManager;