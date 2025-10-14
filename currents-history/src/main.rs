use anyhow::Result;
use clap::{Parser, Subcommand};
use currents_core::types::WeatherData;
use currents_history::{PatternAnalyzer, HistoryConfig};
use currents_storage::{WeatherStorage, StorageConfig};
use std::path::PathBuf;
use std::io::Write;

#[derive(Parser)]
#[command(name = "currents-history")]
#[command(about = "Historical weather tracking and pattern analysis")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long, default_value = "~/.config/currents/config.toml")]
    config: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
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
        output: Option<PathBuf>,
    },
    
    /// Seed database with historical weather data from APIs
    Seed {
        /// Start date (YYYY-MM-DD format)
        #[arg(long)]
        start_date: String,
        
        /// End date (YYYY-MM-DD format)
        #[arg(long)]
        end_date: String,
        
        /// API key for weather service
        #[arg(long)]
        api_key: String,
        
        /// Weather provider (openweathermap, weatherapi)
        #[arg(long, default_value = "weatherapi")]
        provider: String,
        
        /// Location to fetch data for
        #[arg(long)]
        location: String,
        
        /// API daily limit (calls per day)
        #[arg(long, default_value = "1000")]
        daily_limit: u64,
        
        /// Delay between API calls in milliseconds
        #[arg(long, default_value = "100")]
        delay_ms: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Load configuration
    let config_path = expand_tilde(&cli.config)?;
    let config = HistoryConfig::from_file(&config_path)?;
    
    // Initialize storage
    let storage_config = StorageConfig {
        database_path: config.database_path.clone(),
        max_history_days: config.max_history_days,
        enable_compression: config.enable_compression,
        compression_threshold: config.compression_threshold,
    };
    let storage = WeatherStorage::new(&storage_config.database_path, storage_config.clone())?;
    
    match cli.command {
        Commands::History { period, detailed } => {
            let days = parse_period(&period)?;
            let data = storage.get_weather_history(days).await?;
            
            if detailed {
                print_detailed_history(&data);
            } else {
                print_summary_history(&data);
            }
        }
        
        Commands::Analyze { metric, period } => {
            let days = parse_period(&period)?;
            let analyzer = PatternAnalyzer::new(&storage);
            let trends = analyzer.analyze_trends(&metric, days).await?;
            
            print_analysis(&metric, &trends);
        }
        
        Commands::Compare { period } => {
            let days = parse_period(&period)?;
            let analyzer = PatternAnalyzer::new(&storage);
            let comparison = analyzer.compare_to_period(days).await?;
            
            print_comparison(&comparison);
        }
        
        Commands::Trend { metric, period } => {
            let days = parse_period(&period)?;
            let analyzer = PatternAnalyzer::new(&storage);
            let trends = analyzer.analyze_trends(&metric, days).await?;
            
            // TODO: Implement ASCII chart rendering
            println!("Trend visualization for {} over {}:", metric, period);
            println!("(ASCII charts coming soon!)");
            print_analysis(&metric, &trends);
        }
        
        Commands::Export { format, period, output } => {
            let days = parse_period(&period)?;
            let data = storage.get_weather_history(days).await?;
            
            match format.as_str() {
                "json" => {
                    let json = serde_json::to_string_pretty(&data)?;
                    if let Some(path) = output {
                        std::fs::write(&path, json)?;
                        println!("Data exported to {:?}", path);
                    } else {
                        println!("{}", json);
                    }
                }
                "csv" => {
                    let csv = format_weather_data_as_csv(&data);
                    if let Some(path) = output {
                        std::fs::write(&path, csv)?;
                        println!("Data exported to {:?}", path);
                    } else {
                        print!("{}", csv);
                    }
                }
                _ => return Err(anyhow::anyhow!("Unsupported format: {}", format)),
            }
        }
        
        Commands::Seed { start_date, end_date, api_key, provider, location, daily_limit, delay_ms } => {
            println!("Starting historical data seeding...");
            println!("Provider: {}", provider);
            println!("Location: {}", location);
            println!("Date range: {} to {}", start_date, end_date);
            
            // Parse dates
            let start = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
                .context("Invalid start date format. Use YYYY-MM-DD")?
                .and_hms_opt(12, 0, 0)
                .unwrap()
                .and_utc();
            
            let end = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d")
                .context("Invalid end date format. Use YYYY-MM-DD")?
                .and_hms_opt(12, 0, 0)
                .unwrap()
                .and_utc();
            
            if start > end {
                return Err(anyhow::anyhow!("Start date must be before end date"));
            }
            
            // Create weather fetcher
            let fetcher = currents_core::api::WeatherFetcher::new(
                api_key,
                location,
                "metric".to_string(),
                provider,
                daily_limit,
            );
            
            // Fetch historical data
            let historical_data = fetcher.fetch_historical_range(start, end).await?;
            
            println!("Fetched {} historical records", historical_data.len());
            
            // Store data in database
            let mut stored_count = 0;
            let mut error_count = 0;
            
            for weather_data in historical_data {
                match storage.store_weather_data("default", &weather_data).await {
                    Ok(_) => {
                        stored_count += 1;
                        if stored_count % 10 == 0 {
                            print!(".");
                            std::io::stdout().flush().unwrap();
                        }
                    }
                    Err(e) => {
                        error_count += 1;
                        eprintln!("Error storing data for {}: {}", weather_data.timestamp.format("%Y-%m-%d"), e);
                    }
                }
                
                // Add delay between storage operations
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
            
            println!("\nSeeding completed!");
            println!("Successfully stored: {} records", stored_count);
            if error_count > 0 {
                println!("Errors encountered: {} records", error_count);
            }
        }
    }
    
    Ok(())
}

fn expand_tilde(path: &PathBuf) -> Result<PathBuf> {
    if path.starts_with("~") {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(path.strip_prefix("~")?))
    } else {
        Ok(path.clone())
    }
}

fn parse_period(period: &str) -> Result<u32> {
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

fn print_detailed_history(data: &[WeatherData]) {
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

fn print_summary_history(data: &[WeatherData]) {
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

fn print_analysis(metric: &str, trends: &currents_history::TrendData) {
    println!("Weather Analysis: {}", metric);
    println!("  Trend: {}", trends.trend_direction);
    println!("  Change: {:.2} per day", trends.daily_change);
    println!("  Volatility: {:.2}", trends.volatility);
    println!("  Average: {:.2}", trends.average);
    println!("  Min: {:.2}", trends.min);
    println!("  Max: {:.2}", trends.max);
}

fn print_comparison(comparison: &currents_history::WeatherPattern) {
    println!("Weather Comparison:");
    println!("  Current vs Historical:");
    println!("    Temperature: {:.1}°C (avg: {:.1}°C, diff: {:.1}°C)", 
        comparison.current_value, comparison.historical_average, 
        comparison.current_value - comparison.historical_average);
    println!("    Trend: {}", comparison.trend_description);
}

fn format_weather_data_as_csv(data: &[WeatherData]) -> String {
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