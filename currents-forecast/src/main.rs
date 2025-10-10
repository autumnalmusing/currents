use anyhow::Result;
use clap::Parser;
use tracing::info;

mod config;
mod display;

use config::Config;
use currents_core::{WeatherFetcher, ApiStatsTracker};
use display::ForecastFormatter;

#[derive(Parser)]
#[command(name = "currents-forecast")]
#[command(about = "Display weather forecast in a formatted table")]
#[command(version)]
struct Args {
    /// Number of days to show (1-5, default: 5)
    #[arg(short, long, default_value = "5")]
    days: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    
    // Load configuration
    let config = Config::load_from_file()?;
    
    // Check remaining API calls
    if let Ok(tracker) = ApiStatsTracker::with_default_path() {
        if let Ok(remaining) = tracker.remaining_calls(config.weather.api_daily_limit) {
            if remaining == 0 {
                eprintln!("Error: Daily API limit ({} calls) reached.", config.weather.api_daily_limit);
                eprintln!("Limit will reset at midnight UTC.");
                std::process::exit(1);
            }
            eprintln!("API calls remaining today: {}/{}\n", remaining, config.weather.api_daily_limit);
        }
    }
    
    info!("Fetching weather forecast...");
    
    // Create weather fetcher
    let weather_fetcher = WeatherFetcher::new(
        config.weather.api_key.clone(),
        config.weather.location.clone(),
        config.weather.units.clone(),
        config.weather.provider.clone(),
        config.weather.api_daily_limit,
    );
    
    // Fetch forecast
    match weather_fetcher.fetch_forecast().await {
        Ok(mut forecast) => {
            // Limit to requested number of days
            forecast.days.truncate(args.days.min(5));
            
            // Format and display
            let formatted = ForecastFormatter::format(
                &forecast,
                &config.forecast_highlights,
                &config.forecast_display
            );
            println!("{}", formatted);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to fetch forecast: {}", e);
            std::process::exit(1);
        }
    }
}

