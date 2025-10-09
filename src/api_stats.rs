use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiStatsData {
    /// The date (YYYY-MM-DD) in UTC
    date: String,
    /// Number of API calls made on this date
    count: u64,
}

#[derive(Debug, Clone)]
pub struct ApiStatsTracker {
    stats_path: PathBuf,
    data: Arc<Mutex<ApiStatsData>>,
}

impl ApiStatsTracker {
    /// Create a new API stats tracker with a custom path
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

    /// Create with default path (~/.cache/currents/api_stats.json)
    pub fn with_default_path() -> Result<Self> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let stats_path = PathBuf::from(format!("{}/.cache/currents/api_stats.json", home));
        Self::new(stats_path)
    }

    /// Increment the API call counter
    /// Automatically resets if a new day has started (UTC)
    pub fn increment(&self) -> Result<()> {
        let mut data = self.data.lock().unwrap();
        let today = Self::get_current_date_utc();

        // Check if we need to reset (new day)
        if data.date != today {
            data.date = today;
            data.count = 0;
        }

        data.count += 1;

        // Write to disk
        self.save_to_file(&data)?;

        Ok(())
    }

    /// Get the current count for today
    pub fn get_count(&self) -> Result<u64> {
        let mut data = self.data.lock().unwrap();
        let today = Self::get_current_date_utc();

        // If date has changed, reset the counter
        if data.date != today {
            data.date = today;
            data.count = 0;
            self.save_to_file(&data)?;
        }

        Ok(data.count)
    }

    /// Get the current date in YYYY-MM-DD format (UTC)
    fn get_current_date_utc() -> String {
        let now: DateTime<Utc> = Utc::now();
        now.format("%Y-%m-%d").to_string()
    }

    /// Initialize new stats data
    fn initialize_new_stats() -> ApiStatsData {
        ApiStatsData {
            date: Self::get_current_date_utc(),
            count: 0,
        }
    }

    /// Load stats from file
    fn load_from_file(path: &Path) -> Result<ApiStatsData> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read stats file: {:?}", path))?;
        
        let data: ApiStatsData = serde_json::from_str(&contents)
            .with_context(|| "Failed to parse stats file")?;
        
        Ok(data)
    }

    /// Save stats to file
    fn save_to_file(&self, data: &ApiStatsData) -> Result<()> {
        let json = serde_json::to_string_pretty(data)?;
        fs::write(&self.stats_path, json)
            .with_context(|| format!("Failed to write stats file: {:?}", self.stats_path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_date_format() {
        let date = ApiStatsTracker::get_current_date_utc();
        // Should be in YYYY-MM-DD format
        assert!(date.len() == 10);
        assert!(date.chars().nth(4).unwrap() == '-');
        assert!(date.chars().nth(7).unwrap() == '-');
    }

    #[test]
    fn test_increment_and_get() -> Result<()> {
        // Use a temporary file for testing
        let temp_dir = env::temp_dir();
        let stats_path = temp_dir.join("test_api_stats.json");
        
        // Clean up if exists
        let _ = fs::remove_file(&stats_path);

        let tracker = ApiStatsTracker::new(stats_path.clone())?;
        
        // Initial count should be 0
        assert_eq!(tracker.get_count()?, 0);
        
        // Increment and check
        tracker.increment()?;
        assert_eq!(tracker.get_count()?, 1);
        
        tracker.increment()?;
        tracker.increment()?;
        assert_eq!(tracker.get_count()?, 3);

        // Clean up
        let _ = fs::remove_file(&stats_path);
        
        Ok(())
    }
}

