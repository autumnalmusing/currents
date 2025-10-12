//! Cross-location weather analysis and pattern detection

use crate::types::{
    LocationId, LocationCorrelation, WeatherPattern, RegionalAnalysis,
    CorrelationType, PatternType, PatternSeverity
};
use currents_storage::WeatherStorage;
use currents_core::WeatherData;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, debug};

/// Analyzes weather patterns across multiple locations
pub struct CrossLocationAnalyzer {
    storage: Arc<WeatherStorage>,
    correlation_engine: CorrelationEngine,
    pattern_detector: PatternDetector,
}

impl CrossLocationAnalyzer {
    /// Create a new cross-location analyzer
    pub fn new(storage: Arc<WeatherStorage>) -> Self {
        Self {
            storage,
            correlation_engine: CorrelationEngine::new(),
            pattern_detector: PatternDetector::new(),
        }
    }

    /// Analyze correlations between locations
    pub async fn analyze_correlations(
        &self,
        location_ids: &[LocationId],
        time_window: chrono::Duration,
    ) -> Result<Vec<LocationCorrelation>> {
        debug!("Analyzing correlations between {} locations", location_ids.len());
        
        let mut correlations = Vec::new();
        
        // Get weather data for all locations
        let mut location_data: HashMap<LocationId, Vec<WeatherData>> = HashMap::new();
        for location_id in location_ids {
            let data = self.storage.get_weather_data(
                location_id,
                Some(time_window),
            ).await
            .with_context(|| format!("Failed to get data for location: {}", location_id))?;
            location_data.insert(location_id.clone(), data);
        }

        // Calculate correlations between all pairs of locations
        for i in 0..location_ids.len() {
            for j in (i + 1)..location_ids.len() {
                let location_a = &location_ids[i];
                let location_b = &location_ids[j];
                
                if let (Some(data_a), Some(data_b)) = (
                    location_data.get(location_a),
                    location_data.get(location_b)
                ) {
                    let correlation = self.correlation_engine.calculate_correlation(
                        location_a,
                        location_b,
                        data_a,
                        data_b,
                    )?;
                    correlations.push(correlation);
                }
            }
        }

        info!("Found {} correlations between locations", correlations.len());
        Ok(correlations)
    }

    /// Detect weather patterns across locations
    pub async fn detect_patterns(
        &self,
        location_ids: &[LocationId],
        time_window: chrono::Duration,
    ) -> Result<Vec<WeatherPattern>> {
        debug!("Detecting patterns across {} locations", location_ids.len());
        
        // Get weather data for all locations
        let mut location_data: HashMap<LocationId, Vec<WeatherData>> = HashMap::new();
        for location_id in location_ids {
            let data = self.storage.get_weather_data(
                location_id,
                Some(time_window),
            ).await
            .with_context(|| format!("Failed to get data for location: {}", location_id))?;
            location_data.insert(location_id.clone(), data);
        }

        // Detect patterns using the pattern detector
        let patterns = self.pattern_detector.detect_patterns(&location_data)?;
        
        info!("Detected {} weather patterns", patterns.len());
        Ok(patterns)
    }

    /// Perform comprehensive regional analysis
    pub async fn analyze_region(
        &self,
        region_name: &str,
        location_ids: &[LocationId],
        time_window: chrono::Duration,
    ) -> Result<RegionalAnalysis> {
        info!("Performing regional analysis for region: {}", region_name);
        
        // Analyze correlations
        let correlations = self.analyze_correlations(location_ids, time_window).await?;
        
        // Detect patterns
        let patterns = self.detect_patterns(location_ids, time_window).await?;
        
        let analysis = RegionalAnalysis {
            region: region_name.to_string(),
            locations: location_ids.to_vec(),
            patterns,
            correlations,
            analysis_timestamp: chrono::Utc::now(),
        };

        info!("Regional analysis complete for region: {}", region_name);
        Ok(analysis)
    }

    /// Get weather trend comparison between locations
    pub async fn compare_trends(
        &self,
        location_ids: &[LocationId],
        metric: &str,
        time_window: chrono::Duration,
    ) -> Result<HashMap<LocationId, TrendAnalysis>> {
        debug!("Comparing {} trends for {} locations", metric, location_ids.len());
        
        let mut trends = HashMap::new();
        
        for location_id in location_ids {
            let data = self.storage.get_weather_data(
                location_id,
                Some(time_window),
            ).await
            .with_context(|| format!("Failed to get data for location: {}", location_id))?;
            
            let trend = self.analyze_trend(&data, metric)?;
            trends.insert(location_id.clone(), trend);
        }
        
        Ok(trends)
    }

    /// Analyze trend for a specific metric
    fn analyze_trend(&self, data: &[WeatherData], metric: &str) -> Result<TrendAnalysis> {
        if data.is_empty() {
            return Ok(TrendAnalysis {
                direction: TrendDirection::Unknown,
                strength: 0.0,
                confidence: 0.0,
                change_rate: 0.0,
            });
        }

        let values: Vec<f64> = data.iter()
            .filter_map(|d| self.extract_metric_value(d, metric))
            .collect();

        if values.len() < 2 {
            return Ok(TrendAnalysis {
                direction: TrendDirection::Unknown,
                strength: 0.0,
                confidence: 0.0,
                change_rate: 0.0,
            });
        }

        // Calculate linear regression
        let (slope, r_squared) = self.calculate_linear_regression(&values)?;
        
        let direction = if slope > 0.1 {
            TrendDirection::Increasing
        } else if slope < -0.1 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        let strength = r_squared.abs();
        let confidence = if values.len() >= 10 { 0.8 } else { 0.5 };
        let change_rate = slope;

        Ok(TrendAnalysis {
            direction,
            strength,
            confidence,
            change_rate,
        })
    }

    /// Extract metric value from weather data
    fn extract_metric_value(&self, data: &WeatherData, metric: &str) -> Option<f64> {
        match metric {
            "temperature" => Some(data.temperature),
            "humidity" => Some(data.humidity as f64),
            "wind_speed" => Some(data.wind_speed),
            "precipitation_probability" => data.precipitation.as_ref().map(|p| p.probability),
            _ => None,
        }
    }

    /// Calculate linear regression for trend analysis
    fn calculate_linear_regression(&self, values: &[f64]) -> Result<(f64, f64)> {
        let n = values.len() as f64;
        let x_mean = (n - 1.0) / 2.0;
        let y_mean = values.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut denominator = 0.0;
        let mut y_variance = 0.0;

        for (i, &y) in values.iter().enumerate() {
            let x = i as f64;
            let x_diff = x - x_mean;
            let y_diff = y - y_mean;
            
            numerator += x_diff * y_diff;
            denominator += x_diff * x_diff;
            y_variance += y_diff * y_diff;
        }

        let slope = if denominator != 0.0 { numerator / denominator } else { 0.0 };
        let r_squared = if y_variance != 0.0 && denominator != 0.0 {
            (numerator * numerator) / (denominator * y_variance)
        } else {
            0.0
        };

        Ok((slope, r_squared))
    }
}

/// Correlation engine for calculating weather correlations
struct CorrelationEngine;

impl CorrelationEngine {
    fn new() -> Self {
        Self
    }

    fn calculate_correlation(
        &self,
        location_a: &LocationId,
        location_b: &LocationId,
        data_a: &[WeatherData],
        data_b: &[WeatherData],
    ) -> Result<LocationCorrelation> {
        // For now, we'll calculate temperature correlation as an example
        // In a real implementation, this would be more sophisticated
        
        let temp_a: Vec<f64> = data_a.iter().map(|d| d.temperature).collect();
        let temp_b: Vec<f64> = data_b.iter().map(|d| d.temperature).collect();
        
        let correlation = self.pearson_correlation(&temp_a, &temp_b)?;
        
        Ok(LocationCorrelation {
            location_a: location_a.clone(),
            location_b: location_b.clone(),
            correlation_type: CorrelationType::Temperature,
            strength: correlation.abs(),
            confidence: if data_a.len() >= 10 { 0.8 } else { 0.5 },
        })
    }

    fn pearson_correlation(&self, x: &[f64], y: &[f64]) -> Result<f64> {
        if x.len() != y.len() || x.is_empty() {
            return Ok(0.0);
        }

        let n = x.len() as f64;
        let x_mean = x.iter().sum::<f64>() / n;
        let y_mean = y.iter().sum::<f64>() / n;

        let mut numerator = 0.0;
        let mut x_variance = 0.0;
        let mut y_variance = 0.0;

        for (xi, yi) in x.iter().zip(y.iter()) {
            let x_diff = xi - x_mean;
            let y_diff = yi - y_mean;
            
            numerator += x_diff * y_diff;
            x_variance += x_diff * x_diff;
            y_variance += y_diff * y_diff;
        }

        let denominator = (x_variance * y_variance).sqrt();
        Ok(if denominator != 0.0 { numerator / denominator } else { 0.0 })
    }
}

/// Pattern detector for identifying weather patterns
struct PatternDetector;

impl PatternDetector {
    fn new() -> Self {
        Self
    }

    fn detect_patterns(
        &self,
        location_data: &HashMap<LocationId, Vec<WeatherData>>,
    ) -> Result<Vec<WeatherPattern>> {
        let mut patterns = Vec::new();
        
        // Simple pattern detection - in a real implementation this would be more sophisticated
        if location_data.len() >= 2 {
            // Detect temperature gradient pattern
            if let Some(pattern) = self.detect_temperature_gradient(location_data)? {
                patterns.push(pattern);
            }
        }
        
        Ok(patterns)
    }

    fn detect_temperature_gradient(
        &self,
        location_data: &HashMap<LocationId, Vec<WeatherData>>,
    ) -> Result<Option<WeatherPattern>> {
        let mut locations: Vec<_> = location_data.iter().collect();
        if locations.len() < 2 {
            return Ok(None);
        }

        // Calculate average temperatures for each location
        let mut avg_temps: Vec<(LocationId, f64)> = locations.iter()
            .filter_map(|(id, data)| {
                if data.is_empty() {
                    None
                } else {
                    let avg_temp = data.iter().map(|d| d.temperature).sum::<f64>() / data.len() as f64;
                    Some(((*id).clone(), avg_temp))
                }
            })
            .collect();

        // Sort by temperature
        avg_temps.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        if avg_temps.is_empty() {
            return Ok(None);
        }

        let temp_range = avg_temps.last().unwrap().1 - avg_temps.first().unwrap().1;
        
        if temp_range > 10.0 { // Significant temperature gradient
            let affected_locations: Vec<LocationId> = avg_temps.iter().map(|(id, _)| id.clone()).collect();
            
            let pattern = WeatherPattern {
                pattern_id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::TemperatureGradient,
                affected_locations,
                severity: if temp_range > 20.0 { PatternSeverity::High } else { PatternSeverity::Medium },
                description: format!("Temperature gradient of {:.1}°C across {} locations", temp_range, locations.len()),
                detected_at: chrono::Utc::now(),
            };
            
            return Ok(Some(pattern));
        }

        Ok(None)
    }
}

/// Trend analysis result
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub direction: TrendDirection,
    pub strength: f64,      // 0.0 to 1.0
    pub confidence: f64,    // 0.0 to 1.0
    pub change_rate: f64,   // Rate of change per time unit
}

/// Trend direction
#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use currents_core::WeatherData;
    use std::collections::HashMap;

    fn create_test_weather_data(temperature: f64) -> WeatherData {
        WeatherData {
            timestamp: chrono::Utc::now(),
            temperature,
            humidity: 50.0,
            wind_speed: 5.0,
            description: "clear sky".to_string(),
            precipitation: None,
        }
    }

    #[test]
    fn test_trend_analysis() {
        let storage_config = currents_storage::types::StorageConfig::default();
        let analyzer = CrossLocationAnalyzer {
            storage: Arc::new(WeatherStorage::new(":memory:", storage_config).unwrap()),
            correlation_engine: CorrelationEngine::new(),
            pattern_detector: PatternDetector::new(),
        };

        let data = vec![
            create_test_weather_data(10.0),
            create_test_weather_data(12.0),
            create_test_weather_data(14.0),
            create_test_weather_data(16.0),
            create_test_weather_data(18.0),
        ];

        let trend = analyzer.analyze_trend(&data, "temperature").unwrap();
        assert_eq!(trend.direction, TrendDirection::Increasing);
        assert!(trend.strength > 0.0);
    }

    #[test]
    fn test_pearson_correlation() {
        let engine = CorrelationEngine::new();
        
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        
        let correlation = engine.pearson_correlation(&x, &y).unwrap();
        assert!((correlation - 1.0).abs() < 0.001); // Should be perfect positive correlation
    }
}