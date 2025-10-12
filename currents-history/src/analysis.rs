use anyhow::Result;
use chrono::{DateTime, Utc, Datelike};
use currents_storage::{WeatherStorage, DailyWeather};
use statrs::statistics::{Data, Distribution, Min, Max};

/// Pattern analyzer for weather data
pub struct PatternAnalyzer<'a> {
    storage: &'a WeatherStorage,
}

impl<'a> PatternAnalyzer<'a> {
    pub fn new(storage: &'a WeatherStorage) -> Self {
        Self { storage }
    }
    
    /// Analyze trends for a specific metric over time
    pub async fn analyze_trends(&self, metric: &str, days: u32) -> Result<TrendData> {
        let data = self.storage.get_metric_history(metric, days).await?;
        
        if data.is_empty() {
            return Err(anyhow::anyhow!("No data available for metric: {}", metric));
        }
        
        let values: Vec<f64> = data.iter().map(|(_, value)| *value).collect();
        let data_stats = Data::new(values.clone());
        
        // Calculate trend using linear regression
        let trend_slope = self.calculate_linear_regression(&data)?;
        let daily_change = trend_slope * 24.0; // Convert per-hour to per-day
        
        // Calculate volatility (standard deviation)
        let volatility = data_stats.std_dev().unwrap_or(0.0);
        
        // Determine trend direction
        let trend_direction = if trend_slope.abs() < 0.01 {
            "stable".to_string()
        } else if trend_slope > 0.0 {
            "increasing".to_string()
        } else {
            "decreasing".to_string()
        };
        
        Ok(TrendData {
            metric: metric.to_string(),
            trend_direction,
            daily_change,
            volatility,
            average: data_stats.mean().unwrap_or(0.0),
            min: data_stats.min(),
            max: data_stats.max(),
            data_points: data.len(),
            period_days: days,
        })
    }
    
    /// Compare current weather to historical period
    pub async fn compare_to_period(&self, days: u32) -> Result<WeatherPattern> {
        let current = self.storage.get_latest_weather().await?
            .ok_or_else(|| anyhow::anyhow!("No current weather data available"))?;
        
        let historical = self.storage.get_weather_history(days).await?;
        
        if historical.is_empty() {
            return Err(anyhow::anyhow!("No historical data available for comparison"));
        }
        
        // Calculate historical averages
        let hist_temps: Vec<f64> = historical.iter().map(|w| w.temperature).collect();
        let hist_humidity: Vec<f64> = historical.iter().map(|w| w.humidity).collect();
        let hist_wind: Vec<f64> = historical.iter().map(|w| w.wind_speed).collect();
        
        let hist_temp_avg = hist_temps.iter().sum::<f64>() / hist_temps.len() as f64;
        let hist_humidity_avg = hist_humidity.iter().sum::<f64>() / hist_humidity.len() as f64;
        let hist_wind_avg = hist_wind.iter().sum::<f64>() / hist_wind.len() as f64;
        
        // Calculate differences
        let temp_diff = current.temperature - hist_temp_avg;
        let humidity_diff = current.humidity - hist_humidity_avg;
        let wind_diff = current.wind_speed - hist_wind_avg;
        
        // Generate trend description
        let trend_description = self.generate_comparison_description(
            temp_diff, humidity_diff, wind_diff
        );
        
        Ok(WeatherPattern {
            current_value: current.temperature,
            historical_average: hist_temp_avg,
            trend_description,
            temperature_difference: temp_diff,
            humidity_difference: humidity_diff,
            wind_difference: wind_diff,
            comparison_period_days: days,
        })
    }
    
    /// Detect weather patterns and anomalies
    pub async fn detect_patterns(&self, days: u32) -> Result<Vec<WeatherPattern>> {
        let daily_data = self.storage.get_daily_aggregates(days).await?;
        let mut patterns = Vec::new();
        
        if daily_data.len() < 7 {
            return Ok(patterns); // Need at least a week of data
        }
        
        // Analyze temperature patterns
        let temp_pattern = self.analyze_temperature_pattern(&daily_data)?;
        if let Some(pattern) = temp_pattern {
            patterns.push(pattern);
        }
        
        // Analyze precipitation patterns
        let precip_pattern = self.analyze_precipitation_pattern(&daily_data)?;
        if let Some(pattern) = precip_pattern {
            patterns.push(pattern);
        }
        
        // Analyze wind patterns
        let wind_pattern = self.analyze_wind_pattern(&daily_data)?;
        if let Some(pattern) = wind_pattern {
            patterns.push(pattern);
        }
        
        Ok(patterns)
    }
    
    /// Calculate seasonal trends
    pub async fn analyze_seasonal_trends(&self, metric: &str) -> Result<SeasonalTrends> {
        // Get data for the last year
        let data = self.storage.get_metric_history(metric, 365).await?;
        
        if data.is_empty() {
            return Err(anyhow::anyhow!("Insufficient data for seasonal analysis"));
        }
        
        // Group data by month
        let mut monthly_data: std::collections::HashMap<u32, Vec<f64>> = std::collections::HashMap::new();
        
        for (timestamp, value) in &data {
            let month = timestamp.month();
            monthly_data.entry(month).or_default().push(*value);
        }
        
        // Calculate monthly averages
        let mut monthly_averages = Vec::new();
        for month in 1..=12 {
            if let Some(values) = monthly_data.get(&month) {
                let avg = values.iter().sum::<f64>() / values.len() as f64;
                monthly_averages.push((month, avg));
            }
        }
        
        // Find peak and trough months
        let (peak_month, peak_value) = monthly_averages.iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or(&(1, 0.0));
        
        let (trough_month, trough_value) = monthly_averages.iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap_or(&(1, 0.0));
        
        let peak_month = *peak_month;
        let peak_value = *peak_value;
        let trough_month = *trough_month;
        let trough_value = *trough_value;
        
        Ok(SeasonalTrends {
            metric: metric.to_string(),
            monthly_averages,
            peak_month,
            peak_value,
            trough_month,
            trough_value,
            seasonal_variation: peak_value - trough_value,
        })
    }
    
    /// Calculate linear regression slope
    fn calculate_linear_regression(&self, data: &[(DateTime<Utc>, f64)]) -> Result<f64> {
        if data.len() < 2 {
            return Ok(0.0);
        }
        
        let n = data.len() as f64;
        let sum_x: f64 = data.iter().enumerate().map(|(i, _)| i as f64).sum();
        let sum_y: f64 = data.iter().map(|(_, y)| *y).sum();
        let sum_xy: f64 = data.iter().enumerate().map(|(i, (_, y))| i as f64 * y).sum();
        let sum_x2: f64 = data.iter().enumerate().map(|(i, _)| (i as f64).powi(2)).sum();
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
        Ok(slope)
    }
    
    /// Generate human-readable comparison description
    fn generate_comparison_description(&self, temp_diff: f64, humidity_diff: f64, wind_diff: f64) -> String {
        let mut descriptions = Vec::new();
        
        if temp_diff.abs() > 2.0 {
            if temp_diff > 0.0 {
                descriptions.push(format!("{:.1}°C warmer than usual", temp_diff));
            } else {
                descriptions.push(format!("{:.1}°C cooler than usual", temp_diff.abs()));
            }
        }
        
        if humidity_diff.abs() > 10.0 {
            if humidity_diff > 0.0 {
                descriptions.push(format!("{:.0}% more humid than usual", humidity_diff));
            } else {
                descriptions.push(format!("{:.0}% less humid than usual", humidity_diff.abs()));
            }
        }
        
        if wind_diff.abs() > 2.0 {
            if wind_diff > 0.0 {
                descriptions.push(format!("{:.1} m/s windier than usual", wind_diff));
            } else {
                descriptions.push(format!("{:.1} m/s calmer than usual", wind_diff.abs()));
            }
        }
        
        if descriptions.is_empty() {
            "similar to historical average".to_string()
        } else {
            descriptions.join(", ")
        }
    }
    
    /// Analyze temperature patterns
    fn analyze_temperature_pattern(&self, daily_data: &[DailyWeather]) -> Result<Option<WeatherPattern>> {
        if daily_data.len() < 7 {
            return Ok(None);
        }
        
        let temps: Vec<f64> = daily_data.iter().map(|d| d.avg_temp).collect();
        let data_stats = Data::new(temps.clone());
        let avg_temp = data_stats.mean().unwrap_or(0.0);
        let std_dev = data_stats.std_dev().unwrap_or(0.0);
        
        // Check for unusual temperature patterns
        let latest_temp = daily_data.last().unwrap().avg_temp;
        let temp_diff = latest_temp - avg_temp;
        
        if temp_diff.abs() > 2.0 * std_dev {
            let trend_description = if temp_diff > 0.0 {
                "unusually warm".to_string()
            } else {
                "unusually cool".to_string()
            };
            
            return Ok(Some(WeatherPattern {
                current_value: latest_temp,
                historical_average: avg_temp,
                trend_description,
                temperature_difference: temp_diff,
                humidity_difference: 0.0,
                wind_difference: 0.0,
                comparison_period_days: daily_data.len() as u32,
            }));
        }
        
        Ok(None)
    }
    
    /// Analyze precipitation patterns
    fn analyze_precipitation_pattern(&self, _daily_data: &[DailyWeather]) -> Result<Option<WeatherPattern>> {
        // TODO: Implement precipitation pattern analysis
        // This would require precipitation data in DailyWeather
        Ok(None)
    }
    
    /// Analyze wind patterns
    fn analyze_wind_pattern(&self, daily_data: &[DailyWeather]) -> Result<Option<WeatherPattern>> {
        if daily_data.len() < 7 {
            return Ok(None);
        }
        
        let winds: Vec<f64> = daily_data.iter().map(|d| d.avg_wind_speed).collect();
        let data_stats = Data::new(winds.clone());
        let avg_wind = data_stats.mean().unwrap_or(0.0);
        let std_dev = data_stats.std_dev().unwrap_or(0.0);
        
        // Check for unusual wind patterns
        let latest_wind = daily_data.last().unwrap().avg_wind_speed;
        let wind_diff = latest_wind - avg_wind;
        
        if wind_diff.abs() > 2.0 * std_dev {
            let trend_description = if wind_diff > 0.0 {
                "unusually windy".to_string()
            } else {
                "unusually calm".to_string()
            };
            
            return Ok(Some(WeatherPattern {
                current_value: latest_wind,
                historical_average: avg_wind,
                trend_description,
                temperature_difference: 0.0,
                humidity_difference: 0.0,
                wind_difference: wind_diff,
                comparison_period_days: daily_data.len() as u32,
            }));
        }
        
        Ok(None)
    }
}

/// Trend analysis data
#[derive(Debug, Clone)]
pub struct TrendData {
    pub metric: String,
    pub trend_direction: String,
    pub daily_change: f64,
    pub volatility: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
    pub data_points: usize,
    pub period_days: u32,
}

/// Weather pattern analysis
#[derive(Debug, Clone)]
pub struct WeatherPattern {
    pub current_value: f64,
    pub historical_average: f64,
    pub trend_description: String,
    pub temperature_difference: f64,
    pub humidity_difference: f64,
    pub wind_difference: f64,
    pub comparison_period_days: u32,
}

/// Seasonal trend analysis
#[derive(Debug, Clone)]
pub struct SeasonalTrends {
    pub metric: String,
    pub monthly_averages: Vec<(u32, f64)>, // (month, average_value)
    pub peak_month: u32,
    pub peak_value: f64,
    pub trough_month: u32,
    pub trough_value: f64,
    pub seasonal_variation: f64,
}