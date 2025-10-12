use anyhow::Result;
use clap::{Parser, Subcommand};
use currents_core::types::WeatherData;
use crate::{TrendData, WeatherPattern};

/// CLI argument parser for currents-history
#[derive(Parser)]
#[command(name = "currents-history")]
#[command(about = "Historical weather tracking and pattern analysis")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    
    /// Configuration file path
    #[arg(short, long, default_value = "~/.config/currents/config.toml")]
    pub config: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show weather history for a time period
    History {
        /// Time period (e.g., "7d", "30d", "1y")
        #[arg(default_value = "7d")]
        period: String,
        
        /// Show detailed data
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Analyze weather patterns and trends
    Analyze {
        /// Metric to analyze (temperature, humidity, pressure, wind_speed)
        #[arg(default_value = "temperature")]
        metric: String,
        
        /// Time period for analysis
        #[arg(default_value = "30d")]
        period: String,
    },
    
    /// Compare current weather to historical data
    Compare {
        /// Time period to compare against (e.g., "last week", "last month")
        #[arg(default_value = "last week")]
        period: String,
    },
    
    /// Show trend visualization
    Trend {
        /// Metric to visualize
        #[arg(default_value = "temperature")]
        metric: String,
        
        /// Time period
        #[arg(default_value = "30d")]
        period: String,
    },
    
    /// Export historical data
    Export {
        /// Output format (json, csv)
        #[arg(default_value = "json")]
        format: String,
        
        /// Time period
        #[arg(default_value = "30d")]
        period: String,
        
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Show database statistics
    Stats,
    
    /// Clean old data from database
    Clean {
        /// Keep only the last N days
        #[arg(default_value = "365")]
        keep_days: u32,
    },
}

/// CLI output formatters
pub struct OutputFormatter;

impl OutputFormatter {
    /// Print detailed weather history
    pub fn print_detailed_history(data: &[WeatherData]) {
        println!("Weather History ({} records):", data.len());
        println!("{:<20} {:<8} {:<8} {:<8} {:<8} {:<15}", "Timestamp", "Temp", "Humidity", "Wind", "Pressure", "Description");
        println!("{}", "-".repeat(80));
        
        for record in data {
            println!(
                "{:<20} {:<8.1} {:<8.1} {:<8.1} {:<8} {:<15}",
                record.timestamp.format("%Y-%m-%d %H:%M"),
                record.temperature,
                record.humidity,
                record.wind_speed,
                "N/A", // Pressure not in WeatherData yet
                record.description
            );
        }
    }
    
    /// Print summary weather history
    pub fn print_summary_history(data: &[WeatherData]) {
        if data.is_empty() {
            println!("No weather history found.");
            return;
        }
        
        let temps: Vec<f64> = data.iter().map(|d| d.temperature).collect();
        let humidities: Vec<f64> = data.iter().map(|d| d.humidity).collect();
        let winds: Vec<f64> = data.iter().map(|d| d.wind_speed).collect();
        
        let temp_min = temps.iter().cloned().fold(f64::INFINITY, f64::min);
        let temp_max = temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let temp_avg = temps.iter().sum::<f64>() / temps.len() as f64;
        
        let humidity_avg = humidities.iter().sum::<f64>() / humidities.len() as f64;
        let wind_avg = winds.iter().sum::<f64>() / winds.len() as f64;
        
        println!("Weather Summary ({} records):", data.len());
        println!("  Temperature: {:.1}°C (min: {:.1}°C, max: {:.1}°C)", temp_avg, temp_min, temp_max);
        println!("  Humidity: {:.1}%", humidity_avg);
        println!("  Wind Speed: {:.1} m/s", wind_avg);
        println!("  Period: {} to {}", 
            data.first().unwrap().timestamp.format("%Y-%m-%d %H:%M"),
            data.last().unwrap().timestamp.format("%Y-%m-%d %H:%M")
        );
    }
    
    /// Print trend analysis
    pub fn print_analysis(metric: &str, trends: &TrendData) {
        println!("Weather Analysis: {}", metric);
        println!("  Trend: {}", trends.trend_direction);
        println!("  Change: {:.2} per day", trends.daily_change);
        println!("  Volatility: {:.2}", trends.volatility);
        println!("  Average: {:.2}", trends.average);
        println!("  Min: {:.2}", trends.min);
        println!("  Max: {:.2}", trends.max);
        println!("  Data Points: {}", trends.data_points);
        println!("  Period: {} days", trends.period_days);
    }
    
    /// Print weather comparison
    pub fn print_comparison(comparison: &WeatherPattern) {
        println!("Weather Comparison:");
        println!("  Current vs Historical:");
        println!("    Temperature: {:.1}°C (avg: {:.1}°C, diff: {:.1}°C)", 
            comparison.current_value, comparison.historical_average, 
            comparison.current_value - comparison.historical_average);
        println!("    Trend: {}", comparison.trend_description);
        println!("    Humidity diff: {:.1}%", comparison.humidity_difference);
        println!("    Wind diff: {:.1} m/s", comparison.wind_difference);
        println!("    Comparison period: {} days", comparison.comparison_period_days);
    }
    
    /// Print database statistics
    pub fn print_stats(stats: &currents_storage::DatabaseStats) {
        println!("Database Statistics:");
        println!("  Total Records: {}", stats.total_records);
        
        if let Some(earliest) = stats.earliest_timestamp {
            println!("  Earliest Record: {}", earliest.format("%Y-%m-%d %H:%M"));
        }
        
        if let Some(latest) = stats.latest_timestamp {
            println!("  Latest Record: {}", latest.format("%Y-%m-%d %H:%M"));
        }
        
        println!("  Temperature Stats:");
        println!("    Average: {:.1}°C", stats.avg_temperature);
        println!("    Min: {:.1}°C", stats.min_temperature);
        println!("    Max: {:.1}°C", stats.max_temperature);
    }
    
    /// Format weather data as CSV
    pub fn format_weather_data_as_csv(data: &[WeatherData]) -> String {
        let mut csv = String::from("timestamp,temperature,humidity,wind_speed,description\n");
        
        for record in data {
            csv.push_str(&format!(
                "{},{:.1},{:.1},{:.1},{}\n",
                record.timestamp.format("%Y-%m-%d %H:%M:%S"),
                record.temperature,
                record.humidity,
                record.wind_speed,
                record.description
            ));
        }
        
        csv
    }
}

/// Utility functions for CLI
pub struct CliUtils;

impl CliUtils {
    /// Expand tilde in path
    pub fn expand_tilde(path: &str) -> Result<String> {
        if path.starts_with("~") {
            let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
            Ok(home.join(path.strip_prefix("~").unwrap()).to_string_lossy().to_string())
        } else {
            Ok(path.to_string())
        }
    }
    
    /// Parse time period string
    pub fn parse_period(period: &str) -> Result<u32> {
        match period {
            "1d" | "1 day" => Ok(1),
            "7d" | "1 week" | "last week" => Ok(7),
            "30d" | "1 month" | "last month" => Ok(30),
            "90d" | "3 months" => Ok(90),
            "365d" | "1 year" | "last year" => Ok(365),
            _ => {
                // Try to parse as number of days
                if let Ok(days) = period.parse::<u32>() {
                    Ok(days)
                } else {
                    Err(anyhow::anyhow!("Invalid period format: {}. Use formats like '7d', '30d', '1 week', etc.", period))
                }
            }
        }
    }
}