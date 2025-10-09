use currents::api_stats::ApiStatsTracker;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_api_stats_tracker_initialization() {
    let temp_dir = TempDir::new().unwrap();
    let stats_path = temp_dir.path().join("api_stats.json");
    
    let tracker = ApiStatsTracker::new(stats_path.clone()).unwrap();
    
    // Initial count should be 0
    assert_eq!(tracker.get_count().unwrap(), 0);
    
    // After an increment, the file should exist
    tracker.increment().unwrap();
    assert!(stats_path.exists());
}

#[test]
fn test_api_stats_increment() {
    let temp_dir = TempDir::new().unwrap();
    let stats_path = temp_dir.path().join("api_stats.json");
    
    let tracker = ApiStatsTracker::new(stats_path.clone()).unwrap();
    
    tracker.increment().unwrap();
    assert_eq!(tracker.get_count().unwrap(), 1);
    
    tracker.increment().unwrap();
    assert_eq!(tracker.get_count().unwrap(), 2);
    
    tracker.increment().unwrap();
    assert_eq!(tracker.get_count().unwrap(), 3);
}

#[test]
fn test_api_stats_remaining_calls() {
    let temp_dir = TempDir::new().unwrap();
    let stats_path = temp_dir.path().join("api_stats.json");
    
    let tracker = ApiStatsTracker::new(stats_path).unwrap();
    
    // With 1000 limit and 0 calls
    assert_eq!(tracker.remaining_calls(1000).unwrap(), 1000);
    
    // Make some calls
    tracker.increment().unwrap();
    tracker.increment().unwrap();
    tracker.increment().unwrap();
    
    assert_eq!(tracker.remaining_calls(1000).unwrap(), 997);
}

#[test]
fn test_api_stats_at_limit() {
    let temp_dir = TempDir::new().unwrap();
    let stats_path = temp_dir.path().join("api_stats.json");
    
    let tracker = ApiStatsTracker::new(stats_path).unwrap();
    
    // Add calls up to limit
    for _ in 0..10 {
        tracker.increment().unwrap();
    }
    
    assert_eq!(tracker.remaining_calls(10).unwrap(), 0);
    
    // Should be over limit
    for _ in 0..5 {
        tracker.increment().unwrap();
    }
    assert_eq!(tracker.get_count().unwrap(), 15);
}

#[test]
fn test_api_stats_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let stats_path = temp_dir.path().join("api_stats.json");
    
    {
        let tracker = ApiStatsTracker::new(stats_path.clone()).unwrap();
        tracker.increment().unwrap();
        tracker.increment().unwrap();
        tracker.increment().unwrap();
    }
    
    // Create new tracker with same path - should load existing count
    let tracker2 = ApiStatsTracker::new(stats_path).unwrap();
    assert_eq!(tracker2.get_count().unwrap(), 3);
}

#[test]
fn test_api_stats_day_rollover() {
    let temp_dir = TempDir::new().unwrap();
    let stats_path = temp_dir.path().join("api_stats.json");
    
    // Create tracker and add some calls
    let tracker = ApiStatsTracker::new(stats_path.clone()).unwrap();
    tracker.increment().unwrap();
    tracker.increment().unwrap();
    
    assert_eq!(tracker.get_count().unwrap(), 2);
    
    // Manually modify the date in the file to simulate a new day
    let stats_content = fs::read_to_string(&stats_path).unwrap();
    let modified = stats_content.replace(
        &chrono::Utc::now().format("%Y-%m-%d").to_string(),
        "2020-01-01"
    );
    fs::write(&stats_path, modified).unwrap();
    
    // Create new tracker - should reset to 0 for new day
    let tracker2 = ApiStatsTracker::new(stats_path).unwrap();
    assert_eq!(tracker2.get_count().unwrap(), 0);
}

