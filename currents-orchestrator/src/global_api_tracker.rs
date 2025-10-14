//! Global API usage tracking for the orchestrator
//!
//! This module provides global API rate limiting across all locations
//! to prevent exceeding API provider limits at the orchestrator level.

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GlobalApiStatsData {
    /// The date (YYYY-MM-DD) in UTC
    date: String,
    /// Total number of API calls made across all locations on this date
    count: u64,
    /// Breakdown by location (for monitoring/debugging)
    location_breakdown: std::collections::HashMap<String, u64>,
}

#[derive(Debug, Clone)]
pub struct GlobalApiTracker {
    stats_path: PathBuf,
    data: Arc<Mutex<GlobalApiStatsData>>,
}

impl GlobalApiTracker {
    /// Create a new global API tracker with a custom path
    pub fn new(stats_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = stats_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create stats directory: {:?}", parent))?;
        }

        // Load or initialize stats
        let data = if stats_path.exists() {
            Self::load_from_file(&stats_path)?
        } else {
            Self::initialize_new_stats()
        };

        Ok(Self {
            stats_path,
            data: Arc::new(Mutex::new(data)),
        })
    }

    /// Create with default path (~/.cache/currents/global_api_stats.json)
    pub fn with_default_path() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let stats_path = PathBuf::from(format!("{}/.cache/currents/global_api_stats.json", home));
        Self::new(stats_path)
    }

    /// Increment the global API call counter for a specific location
    /// Automatically resets if a new day has started (UTC)
    pub fn increment(&self, location_id: &str) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        let today = Self::get_current_date_utc();

        // Check if we need to reset (new day)
        if data.date != today {
            info!("New day detected, resetting global API counter");
            data.date = today;
            data.count = 0;
            data.location_breakdown.clear();
        }

        data.count += 1;
        *data.location_breakdown.entry(location_id.to_string()).or_insert(0) += 1;

        // Write to disk
        self.save_to_file(&data)?;

        Ok(())
    }

    /// Get the current global count for today
    pub fn get_count(&self) -> Result<u64> {
        let mut data = self.data.lock().unwrap();
        let today = Self::get_current_date_utc();

        // If date has changed, reset the counter
        if data.date != today {
            data.date = today;
            data.count = 0;
            data.location_breakdown.clear();
            self.save_to_file(&data)?;
        }

        Ok(data.count)
    }

    /// Check if we can make an API call without exceeding the global daily limit
    /// Returns true if we're under the limit, false otherwise
    pub fn can_make_call(&self, limit: u64) -> Result<bool> {
        let count = self.get_count()?;
        Ok(count < limit)
    }

    /// Get how many calls are remaining today globally
    pub fn remaining_calls(&self, limit: u64) -> Result<u64> {
        let count = self.get_count()?;
        Ok(limit.saturating_sub(count))
    }

    /// Get the breakdown of API calls by location
    pub fn get_location_breakdown(&self) -> Result<std::collections::HashMap<String, u64>> {
        let data = self.data.lock().unwrap();
        Ok(data.location_breakdown.clone())
    }

    /// Get the current date in YYYY-MM-DD format (UTC)
    fn get_current_date_utc() -> String {
        let now: DateTime<Utc> = Utc::now();
        now.format("%Y-%m-%d").to_string()
    }

    /// Initialize new stats data
    fn initialize_new_stats() -> GlobalApiStatsData {
        GlobalApiStatsData {
            date: Self::get_current_date_utc(),
            count: 0,
            location_breakdown: std::collections::HashMap::new(),
        }
    }

    /// Load stats from file
    fn load_from_file(path: &Path) -> Result<GlobalApiStatsData> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read stats file: {:?}", path))?;
        
        let data: GlobalApiStatsData = serde_json::from_str(&contents)
            .with_context(|| "Failed to parse stats file")?;
        
        Ok(data)
    }

    /// Save stats to file
    fn save_to_file(&self, data: &GlobalApiStatsData) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        fs::write(&self.stats_path, json)
            .with_context(|| format!("Failed to write stats file: {:?}", self.stats_path))?;
        Ok(())
    }

    /// Log current API usage statistics
    pub fn log_usage_stats(&self, limit: u64) -> Result<()> {
        let count = self.get_count()?;
        let remaining = self.remaining_calls(limit)?;
        let breakdown = self.get_location_breakdown()?;

        info!(
            "Global API usage: {}/{} calls used, {} remaining",
            count, limit, remaining
        );

        if !breakdown.is_empty() {
            info!("API usage by location: {:?}", breakdown);
        }

        if remaining < 10 {
            warn!("Low API quota remaining: {} calls left", remaining);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_increment_and_get() -> Result<()> {
        // Use a temporary file for testing
        let temp_dir = env::temp_dir();
        let stats_path = temp_dir.join("test_global_api_stats.json");
        
        // Clean up if exists
        let _ = fs::remove_file(&stats_path);

        let tracker = GlobalApiTracker::new(stats_path.clone())?;
        
        // Initial count should be 0
        assert_eq!(tracker.get_count()?, 0);
        
        // Increment for different locations
        tracker.increment("london")?;
        tracker.increment("tokyo")?;
        tracker.increment("london")?;
        
        assert_eq!(tracker.get_count()?, 3);
        
        let breakdown = tracker.get_location_breakdown()?;
        assert_eq!(breakdown.get("london"), Some(&2));
        assert_eq!(breakdown.get("tokyo"), Some(&1));

        // Clean up
        let _ = fs::remove_file(&stats_path);
        
        Ok(())
    }

    #[test]
    fn test_can_make_call() -> Result<()> {
        let temp_dir = env::temp_dir();
        let stats_path = temp_dir.join("test_can_make_call.json");
        let _ = fs::remove_file(&stats_path);

        let tracker = GlobalApiTracker::new(stats_path.clone())?;
        
        // Should be able to make calls under limit
        assert!(tracker.can_make_call(10)?);
        
        // Increment to limit
        for _ in 0..10 {
            tracker.increment("test")?;
        }
        
        // Should not be able to make more calls
        assert!(!tracker.can_make_call(10)?);
        
        // Should be able to make calls with higher limit
        assert!(tracker.can_make_call(15)?);

        let _ = fs::remove_file(&stats_path);
        Ok(())
    }
}
