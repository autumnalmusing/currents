use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use currents::config::Config;
use currents::daemon::WeatherAlertDaemon;

#[derive(Parser)]
#[command(name = "currents")]
#[command(about = "A daemon for sending weather pattern alerts")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "~/.config/currents/config.toml")]
    config: PathBuf,
    
    /// Run in foreground (for debugging)
    #[arg(short, long)]
    foreground: bool,
    
    /// Send a test notification and exit
    #[arg(long)]
    test_notification: bool,
    
    /// Send a custom test notification with title and message
    #[arg(long, value_names = ["TITLE", "MESSAGE"])]
    test_custom: Vec<String>,
    
    /// Send a simple test notification without config file
    #[arg(long)]
    test_simple: bool,

    /// Output Waybar JSON from cache and exit
    #[arg(long)]
    waybar: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    
    // Expand tilde in config path
    let config_path = if args.config.starts_with("~") {
        let home = std::env::var("HOME")?;
        PathBuf::from(args.config.to_string_lossy().replacen("~", &home, 1))
    } else {
        args.config
    };

    // Handle simple test notification (no config required)
    if args.test_simple {
        info!("Sending simple test notification...");
        use currents::notifications::NotificationManager;
        use currents::config::NotificationConfig;
        
        let notif_config = NotificationConfig {
            urgency: "normal".to_string(),
            timeout: 5000,
            sound: true,
        };
        
        let notification_manager = NotificationManager::new(notif_config);
        notification_manager.send_test_notification().await?;
        info!("Simple test notification sent successfully");
        return Ok(());
    }
    
    // If only printing waybar JSON, try reading config for cache path, but fall back to default
    if args.waybar {
        // Try load config to get cache path; if it fails, use default CacheConfig
        let config = Config::load(&config_path).ok();
        let cache_path = config
            .as_ref()
            .map(|c| c.cache.path.clone())
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                format!("{}/.cache/currents/weather.json", home)
            });
        match std::fs::read_to_string(&cache_path) {
            Ok(contents) => {
                println!("{}", contents);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Waybar cache not available: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Load configuration
    let config = Config::load(&config_path)?;
    
    // Handle test notifications
    if args.test_notification {
        info!("Sending test notification...");
        let daemon = WeatherAlertDaemon::new(config).await?;
        daemon.send_test_notification().await?;
        info!("Test notification sent successfully");
        return Ok(());
    }
    
    if !args.test_custom.is_empty() {
        if args.test_custom.len() < 2 {
            eprintln!("Error: --test-custom requires both TITLE and MESSAGE");
            eprintln!("Usage: --test-custom \"Title\" \"Message\"");
            std::process::exit(1);
        }
        
        let title = &args.test_custom[0];
        let message = &args.test_custom[1];
        info!("Sending custom test notification: {} - {}", title, message);
        
        let daemon = WeatherAlertDaemon::new(config).await?;
        daemon.send_custom_notification(title, message).await?;
        info!("Custom test notification sent successfully");
        return Ok(());
    }
    
    info!("Starting currents daemon");
    info!("Loading configuration from: {:?}", config_path);
    
    // Create and run daemon
    let daemon = WeatherAlertDaemon::new(config).await?;
    
    if args.foreground {
        info!("Running in foreground mode");
        daemon.run_foreground().await?;
    } else {
        info!("Running as daemon");
        daemon.run().await?;
    }

    Ok(())
}
