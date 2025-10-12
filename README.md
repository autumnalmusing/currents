# Currents - Modular Weather Monitoring System

A fast, lightweight weather monitoring system written in Rust with a modular plugin architecture. The core daemon monitors weather conditions and sends desktop notifications based on custom alert rules.

## Architecture

Currents is built as a Cargo workspace with independent, composable crates:

- **`currents-core`** - Shared library with API clients and data types
- **`currents-daemon`** - Lightweight background daemon for weather alerts
- **`currents-forecast`** - Standalone forecast display tool
- **`currents-history`** - Historical weather tracking and pattern analysis
- **`currents-storage`** - Centralized data storage with multi-location support
- **`currents-orchestrator`** - Multi-location monitoring orchestrator

Each tool can be installed independently based on your needs.

## Features

### Core Daemon (`currents`)
- **Lightweight Monitoring**: Efficient background process with minimal resource usage
- **Weather Alerts**: Custom notification rules based on weather conditions
- **API Rate Limiting**: Automatic tracking to stay under free tier limits (prevents charges)
- **Cache for Waybar**: Exports weather data for status bar integration
- **Automatic Data Collection**: Optional automatic logging of weather data for historical analysis
- **Multiple Weather Providers**: Supports OpenWeatherMap and WeatherAPI

### Forecast Tool (`currents-forecast`)
- **5-Day Weather Forecast**: Beautiful formatted table with comprehensive data
- **Configurable Display**: Show/hide columns (pressure, UV, AQI, visibility, etc.)
- **Color Highlighting**: Threshold-based highlighting with custom colors
- **Independent**: Runs standalone, doesn't require daemon

### History Tool (`currents-history`)
- **Weather History Tracking**: SQLite database storage for historical weather data
- **Pattern Analysis**: Trend detection and statistical analysis of weather patterns
- **Data Export**: Export historical data in JSON or CSV formats
- **Trend Visualization**: ASCII charts for weather trend visualization
- **Comparison Tools**: Compare current weather to historical periods
- **Seasonal Analysis**: Identify seasonal patterns and variations

### Orchestrator (`currents-orchestrator`)
- **Multi-Location Monitoring**: Monitor weather across multiple cities/regions simultaneously
- **Cross-Location Analysis**: Detect correlations and patterns between locations
- **Independent Collectors**: Each location runs its own collector process
- **Health Monitoring**: Automatic recovery and error handling for collectors
- **Centralized Coordination**: Unified management of multiple weather monitoring processes

### Storage (`currents-storage`)
- **Centralized Data Storage**: Single database with location tagging
- **Multi-Location Support**: Store and query weather data from multiple locations
- **Data Migration**: Schema management and database migrations
- **Efficient Queries**: Optimized data access patterns for historical analysis

### Shared Features
- **Flexible Configuration**: Single TOML config file shared across all tools
- **Rich Data**: Temperature, humidity, wind, pressure, UV, AQI, cloud cover, and more
- **Provider Support**: OpenWeatherMap and WeatherAPI with automatic field mapping

## Installation

### Option 1: NixOS / Home Manager (Recommended for Nix users)

If you're using NixOS or home-manager, you can configure currents declaratively:

1. **Add to your flake inputs**:
   ```nix
   inputs.currents.url = "github:autumnalmusing/currents";
   ```

2. **Import the home-manager module**:
   ```nix
   imports = [
     inputs.currents.homeManagerModules.currents
   ];
   ```

3. **Configure in your home.nix**:
   ```nix
   services.currents = {
     enable = true;
     
     weather = {
       # Use apiKeyFile for better security (recommended)
       apiKeyFile = "/run/secrets/openweathermap-api-key";
       # Or use apiKey directly (not recommended for production)
       # apiKey = "your-api-key-here";
       
       location = "Denver,US";
       units = "metric";
       provider = "openweathermap";
     };
     
     alerts = [
       {
         name = "hot-weather";
         enabled = true;
         message = "Temperature is above 30°C!";
         repeat = "once";
         condition.temperature.min = 30.0;
       }
     ];
     
     # Customize forecast colors
     forecastHighlights.temperature = {
       high = 30.0;
       highColor = "#f38ba8";  # Hex colors or named colors
       low = 0.0;
       lowColor = "#89b4fa";
     };
   };
   ```

See `nix/example-config.nix` for a complete configuration example with all available options.

The service will automatically:
- Install the currents package
- Generate `~/.config/currents/config.toml` from your Nix config
- Set up and start the systemd user service
- Handle resource limits and security settings

### Option 2: Install from source

**Install all tools:**
```bash
# Build everything
cargo build --release --workspace

# Install daemon (required for alerts)
cargo install --path currents-daemon

# Install forecast tool (optional)
cargo install --path currents-forecast

# Install history tool (optional)
cargo install --path currents-history
```

**Install only what you need:**
```bash
# Just the daemon (minimal, alerts only)
cargo install --path currents-daemon

# Just the forecast tool (no daemon)
cargo install --path currents-forecast

# Just the history tool (no daemon)
cargo install --path currents-history
```

**Setup the daemon service:**
```bash
# Install systemd service
./install-service.sh

# Configure
cp config.toml ~/.config/currents/config.toml
# Edit with your API key and preferences

# Start the daemon
sudo systemctl enable currents@$USER
sudo systemctl start currents@$USER
```

## Configuration

The daemon uses TOML configuration format.

### Configuration Files
- `config.toml` - TOML configuration example

### Weather Providers

- **OpenWeatherMap**: Free tier available (1000 calls/day), requires API key
- **WeatherAPI**: Free tier available (1000 calls/day), requires API key

### API Rate Limiting

Configure the maximum daily API calls in your config file:

```toml
[weather]
api_daily_limit = 1000  # Default: 1000 calls per day
```

- **Default: 1000 calls/day** - Stays within free tier limits
- **Increase if needed** - Set higher if you're willing to pay for more API calls
- **Automatic protection** - Prevents charges by blocking requests after limit is reached
- **Resets at midnight UTC** - Counter resets automatically each day

### Alert Conditions

- **Temperature**: Min/max temperature thresholds
- **Humidity**: Humidity level ranges
- **Wind Speed**: Wind speed thresholds
- **Precipitation**: Intensity and probability
- **Description**: Keyword matching on weather descriptions

### Alert Repetition Control

Control how often alerts repeat using the `repeat` setting:

- **`"once"`** (default): Alert only on first trigger, silent while condition persists, alerts again if condition clears and re-triggers
- **`"always"`**: Alert every polling interval while condition is true (use sparingly!)
- **`"3600"`** (or any number): Alert every N seconds while condition persists (e.g., every hour)

**Examples:**
```toml
[[alerts]]
name = "hot-weather"
repeat = "once"      # Default: only alert when temperature first exceeds threshold
message = "It's hot!"
[alerts.condition.temperature]
min = 30.0

[[alerts]]
name = "cold-weather"  
repeat = "3600"      # Alert every hour while cold
message = "Still cold!"
[alerts.condition.temperature]
max = 5.0
```

## Usage

### Daemon Commands

```bash
# Run in foreground (for testing)
currents --foreground

# Test notifications
currents --test-simple                                 # No config required
currents --test-notification                           # Test with your config
currents --test-custom "Title" "Message"              # Custom notification

# Check API usage
currents --api-stats                                   # Shows calls made today
currents --output                                      # View cached weather data

# Service management
systemctl --user status currents                       # Check daemon status
journalctl --user -u currents -f                      # View logs
systemctl --user restart currents                     # Restart daemon
```

### Forecast Display

The forecast tool is a separate binary:

```bash
# Show 5-day forecast
currents-forecast

# Limit number of days
currents-forecast --days 3

# Help
currents-forecast --help
```

**Forecast Features:**
- Daily high/low temperatures, humidity, wind speed
- Pressure, visibility, UV index, cloud cover (configurable)
- Air Quality Index (WeatherAPI only)
- Wind direction and gusts
- Color highlighting based on thresholds
- Automatically checks API rate limits

**Example output:**
```
API calls remaining today: 985/1000

╭─────────┬────────────────┬─────────┬──────────┬────────┬────────╮
│  Date   │    Weather     │  Temp   │ Humidity │  Wind  │ Precip │
├─────────┼────────────────┼─────────┼──────────┼────────┼────────┤
│ Fri 10/10│ scattered clouds│ 11°-18° │   72%    │ 2.0m/s │   0%   │
│ Sat 10/11│   light rain   │ 12°-16° │   85%    │ 4.5m/s │  60%   │
│ Sun 10/12│   clear sky    │ 10°-19° │   65%    │ 1.8m/s │   5%   │
╰─────────┴────────────────┴─────────┴──────────┴────────┴────────╯
```

### Configuration Display Options

Control which columns appear in the forecast:

```toml
[forecast_display]
show_date = true
show_weather = true
show_temp = true
show_humidity = true
show_wind = true
show_precip = true
show_pressure = false      # Atmospheric pressure (optional)
show_visibility = false    # Visibility distance (optional)
show_uv = false           # UV index (optional)
show_clouds = false       # Cloud coverage (optional)
show_wind_dir = false     # Wind direction (optional)
show_aqi = false          # Air quality index (optional)
```

### Weather History Analysis

The history tool provides comprehensive weather data analysis:

```bash
# Show weather history for different time periods
currents-history history 7d              # Last 7 days
currents-history history 30d --detailed  # Last 30 days with detailed data
currents-history history 1y              # Last year

# Analyze weather patterns and trends
currents-history analyze temperature 30d # Temperature trends over 30 days
currents-history analyze humidity 7d     # Humidity analysis
currents-history analyze wind_speed 90d  # Wind speed patterns

# Compare current weather to historical data
currents-history compare "last week"     # Compare to last week
currents-history compare "last month"    # Compare to last month

# Visualize trends with ASCII charts
currents-history trend temperature 30d   # Temperature trend chart
currents-history trend humidity 7d       # Humidity trend chart

# Export historical data
currents-history export json 30d         # Export as JSON
currents-history export csv 7d --output weather.csv  # Export as CSV

# Database management
currents-history stats                   # Show database statistics
currents-history clean 365              # Keep only last 365 days
```

**History Features:**
- **SQLite Storage**: Efficient local database for weather data
- **Pattern Detection**: Automatic identification of weather trends and anomalies
- **Statistical Analysis**: Mean, min, max, volatility calculations
- **Seasonal Analysis**: Monthly pattern identification
- **Data Export**: JSON and CSV export capabilities
- **ASCII Visualization**: Terminal-based trend charts
- **Comparison Tools**: Compare current conditions to historical periods

**Example Analysis Output:**
```
Weather Analysis: temperature
  Trend: increasing
  Change: 0.5 per day
  Volatility: 2.3
  Average: 18.5
  Min: 12.0
  Max: 25.0
  Data Points: 720
  Period: 30 days
```

### History Configuration

Enable weather history tracking in your config:

```toml
[history]
enabled = true
database_path = "~/.config/currents/weather_history.db"
max_history_days = 365
auto_collect = true
collection_interval = 3600
enable_compression = true
compression_threshold = 30
```

## Contributing

For development information, workspace structure, and contribution guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

This project is licensed under the [Peer Production License](https://wiki.p2pfoundation.net/Peer_Production_License).

The Peer Production License is a copyfarleft license that allows:
- ✅ Use, modification, and distribution by worker-owned cooperatives
- ✅ Use by non-profit organizations and individuals
- ❌ Commercial use by traditional for-profit corporations that employ wage labor

This ensures the software remains a commons for those who contribute to the commons.
