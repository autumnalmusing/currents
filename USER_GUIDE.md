# Currents User Guide

**Complete guide to using the Currents weather monitoring system**

## Table of Contents

1. [Quick Start](#quick-start)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Single Location Monitoring](#single-location-monitoring)
5. [Multi-Location Monitoring](#multi-location-monitoring)
6. [Weather Analysis](#weather-analysis)
7. [Troubleshooting](#troubleshooting)
8. [Advanced Usage](#advanced-usage)

---

## Quick Start

### For Single Location Users
```bash
# Install the daemon
cargo install --path currents-daemon

# Configure
cp config.toml ~/.config/currents/config.toml
# Edit with your API key and location

# Start the service
./install-service.sh
sudo systemctl enable currents@$USER
sudo systemctl start currents@$USER
```

### For Multi-Location Users
```bash
# Install the orchestrator
cargo install --path currents-orchestrator

# Configure multiple locations
cp config.orchestrator.toml.example ~/.config/currents/orchestrator.toml
# Edit with your locations and API keys

# Start the orchestrator
currents-orchestrator ~/.config/currents/orchestrator.toml
```

---

## Installation

### Option 1: NixOS / Home Manager (Recommended)

If you're using NixOS or home-manager, you can configure currents declaratively:

```nix
# In your flake.nix
inputs.currents.url = "github:autumnalmusing/currents";

# In your home.nix
imports = [ inputs.currents.homeManagerModules.currents ];

services.currents = {
  enable = true;
  weather = {
    apiKeyFile = "/run/secrets/openweathermap-api-key";
    location = "Denver,US";
    units = "metric";
    provider = "openweathermap";
  };
  alerts = [
    {
      name = "hot-weather";
      enabled = true;
      message = "Temperature is above 30°C!";
      condition.temperature.min = 30.0;
    }
  ];
};
```

### Option 2: Build from Source

**Install all components:**
```bash
# Clone the repository
git clone https://github.com/autumnalmusing/currents.git
cd currents

# Build everything
cargo build --release --workspace

# Install components as needed
cargo install --path currents-daemon        # Single location monitoring
cargo install --path currents-orchestrator # Multi-location monitoring
cargo install --path currents-forecast     # Weather forecast display
cargo install --path currents-history      # Weather analysis
```

**Install only what you need:**
```bash
# Just single location monitoring
cargo install --path currents-daemon

# Just multi-location monitoring
cargo install --path currents-orchestrator

# Just forecast display
cargo install --path currents-forecast

# Just weather analysis
cargo install --path currents-history
```

---

## Configuration

### Single Location Configuration

Create `~/.config/currents/config.toml`:

```toml
[weather]
api_key = "your-openweathermap-api-key"
location = "Denver,US"
units = "metric"
provider = "openweathermap"
polling_interval = 1800  # 30 minutes

[alerts]
[[alerts]]
name = "hot-weather"
enabled = true
message = "Temperature is above 30°C!"
repeat = "once"
[alerts.condition.temperature]
min = 30.0

[[alerts]]
name = "rain-alert"
enabled = true
message = "Rain is expected!"
repeat = "once"
[alerts.condition.precipitation]
probability_min = 0.5

[history]
enabled = true
database_path = "~/.config/currents/weather_history.db"
max_history_days = 365
auto_collect = true
collection_interval = 3600
```

### Multi-Location Configuration

Create `~/.config/currents/orchestrator.toml`:

```toml
[orchestrator]
storage_path = "~/.config/currents/orchestrator.db"
analysis_interval = 3600
health_check_interval = 300

[orchestrator.locations.london]
name = "London, UK"
coordinates = [51.5074, -0.1278]
weather.api_key = "your-openweathermap-api-key"
weather.provider = "openweathermap"
weather.units = "metric"
weather.collection_interval = 1800
collection_strategy = "interval"

[orchestrator.locations.tokyo]
name = "Tokyo, JP"
coordinates = [35.6762, 139.6503]
weather.api_key = "your-openweathermap-api-key"
weather.provider = "openweathermap"
weather.units = "metric"
weather.collection_interval = 1800
collection_strategy = "interval"

[orchestrator.locations.denver]
name = "Denver, US"
coordinates = [39.7392, -104.9903]
weather.api_key = "your-openweathermap-api-key"
weather.provider = "openweathermap"
weather.units = "imperial"
weather.collection_interval = 1800
collection_strategy = "interval"
```

### Weather Providers

Currents supports multiple weather providers:

**OpenWeatherMap** (Recommended)
- Free tier: 1000 calls/day
- Comprehensive data
- Global coverage
- Get API key: https://openweathermap.org/api

**WeatherAPI**
- Free tier: 1000 calls/day
- Additional features (AQI, UV index)
- Get API key: https://weatherapi.com

### API Rate Limiting

Configure daily limits to stay within free tiers:

```toml
[weather]
api_daily_limit = 1000  # Default: 1000 calls per day
```

- **Automatic protection**: Prevents charges by blocking requests after limit
- **Resets at midnight UTC**: Counter resets automatically each day
- **Per-provider limits**: Each provider has its own counter

---

## Single Location Monitoring

### Basic Setup

1. **Configure the daemon:**
   ```bash
   cp config.toml ~/.config/currents/config.toml
   # Edit with your API key and location
   ```

2. **Install the service:**
   ```bash
   ./install-service.sh
   ```

3. **Start the service:**
   ```bash
   sudo systemctl enable currents@$USER
   sudo systemctl start currents@$USER
   ```

### Monitoring Commands

```bash
# Check daemon status
systemctl --user status currents

# View logs
journalctl --user -u currents -f

# Test notifications
currents --test-notification

# Check API usage
currents --api-stats

# View cached weather data
currents --output
```

### Alert Configuration

Configure weather alerts in your `config.toml`:

```toml
[[alerts]]
name = "hot-weather"
enabled = true
message = "Temperature is above 30°C!"
repeat = "once"  # Alert only when condition first triggers
[alerts.condition.temperature]
min = 30.0

[[alerts]]
name = "cold-weather"
enabled = true
message = "Temperature is below 0°C!"
repeat = "3600"  # Alert every hour while cold
[alerts.condition.temperature]
max = 0.0

[[alerts]]
name = "windy-weather"
enabled = true
message = "Strong winds detected!"
repeat = "once"
[alerts.condition.wind_speed]
min = 15.0

[[alerts]]
name = "rain-alert"
enabled = true
message = "Rain is expected!"
repeat = "once"
[alerts.condition.precipitation]
probability_min = 0.5
```

**Alert Repetition Options:**
- `"once"`: Alert only when condition first triggers
- `"always"`: Alert every polling interval (use sparingly!)
- `"3600"`: Alert every N seconds while condition persists

---

## Multi-Location Monitoring

### Overview

The orchestrator enables monitoring multiple locations simultaneously with:
- **Independent collectors** for each location
- **Cross-location analysis** and correlation detection
- **Centralized storage** with location tagging
- **Health monitoring** and automatic recovery

### Basic Setup

1. **Configure the orchestrator:**
   ```bash
   cp config.orchestrator.toml.example ~/.config/currents/orchestrator.toml
   # Edit with your locations and API keys
   ```

2. **Start the orchestrator:**
   ```bash
   currents-orchestrator ~/.config/currents/orchestrator.toml
   ```

### Location Management

**Adding a new location:**
```toml
[orchestrator.locations.new_york]
name = "New York, US"
coordinates = [40.7128, -74.0060]
weather.api_key = "your-api-key"
weather.provider = "openweathermap"
weather.units = "imperial"
weather.collection_interval = 1800
collection_strategy = "interval"
```

**Collection Strategies:**
- `"interval"`: Collect at regular intervals
- `"threshold"`: Collect when conditions change significantly
- `"hybrid"`: Combination of both

### Cross-Location Analysis

The orchestrator automatically analyzes correlations between locations:

```bash
# View orchestrator status
currents-orchestrator --status

# View cross-location correlations
currents-orchestrator --correlations

# View weather patterns
currents-orchestrator --patterns

# Export analysis data
currents-orchestrator --export analysis.json
```

### Health Monitoring

The orchestrator monitors collector health:

```bash
# Check collector status
currents-orchestrator --health

# View process information
currents-orchestrator --processes

# Restart failed collectors
currents-orchestrator --restart-failed
```

### Demo and Testing

Test the multi-location system:

```bash
# Run the demo (requires OPENWEATHER_API_KEY)
export OPENWEATHER_API_KEY="your-api-key"
./demo-phase3.sh
```

The demo will:
- Start collectors for London, Tokyo, and Denver
- Run for 3 minutes collecting data
- Show cross-location analysis results
- Clean up automatically

---

## Weather Analysis

### Historical Data Seeding

Seed your database with historical weather data from paid APIs:

```bash
# Seed with WeatherAPI (recommended - supports up to 365 days)
currents-history seed \
  --start-date 2023-01-01 \
  --end-date 2024-01-01 \
  --api-key YOUR_WEATHERAPI_KEY \
  --provider weatherapi \
  --location "London,UK" \
  --daily-limit 1000 \
  --delay-ms 100

# Seed with OpenWeatherMap (limited to 5 days historical)
currents-history seed \
  --start-date 2024-01-01 \
  --end-date 2024-01-05 \
  --api-key YOUR_OPENWEATHER_KEY \
  --provider openweathermap \
  --location "51.5074,0.1278" \
  --daily-limit 1000
```

**Important Notes:**
- WeatherAPI is recommended for historical data (up to 365 days)
- OpenWeatherMap historical API is limited to 5 days
- Both require paid API keys
- The seeding process includes rate limiting and progress tracking
- Data is stored in the same SQLite database used by the daemon

### Historical Data

Enable weather history tracking:

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

### Analysis Commands

```bash
# Show weather history
currents-history history 7d              # Last 7 days
currents-history history 30d --detailed  # Last 30 days with details
currents-history history 1y              # Last year

# Analyze patterns
currents-history analyze temperature 30d # Temperature trends
currents-history analyze humidity 7d     # Humidity analysis
currents-history analyze wind_speed 90d  # Wind patterns

# Compare to historical data
currents-history compare "last week"     # Compare to last week
currents-history compare "last month"    # Compare to last month

# Visualize trends
currents-history trend temperature 30d     # Temperature chart
currents-history trend humidity 7d         # Humidity chart

# Export data
currents-history export json 30d         # Export as JSON
currents-history export csv 7d --output weather.csv  # Export as CSV

# Database management
currents-history stats                   # Database statistics
currents-history clean 365               # Keep only last 365 days

# Historical data seeding (requires paid API key)
currents-history seed \
  --start-date 2023-01-01 \
  --end-date 2024-01-01 \
  --api-key YOUR_API_KEY \
  --provider weatherapi \
  --location "London,UK" \
  --daily-limit 1000
```

### Forecast Display

```bash
# Show 5-day forecast
currents-forecast

# Limit number of days
currents-forecast --days 3

# Show all available data
currents-forecast --all-columns
```

**Forecast Features:**
- Daily high/low temperatures, humidity, wind speed
- Pressure, visibility, UV index, cloud cover
- Air Quality Index (WeatherAPI only)
- Wind direction and gusts
- Color highlighting based on thresholds

### Cross-Location Analysis

With the orchestrator, you can analyze weather patterns across multiple locations:

```bash
# View correlations between locations
currents-orchestrator --correlations

# Detect weather patterns
currents-orchestrator --patterns

# Compare locations
currents-orchestrator --compare london tokyo

# Export regional analysis
currents-orchestrator --export regional_analysis.json
```

---

## Troubleshooting

### Common Issues

**1. API Key Issues**
```bash
# Check API key configuration
currents --api-stats

# Test API connection
currents --test-notification
```

**2. Service Not Starting**
```bash
# Check service status
systemctl --user status currents

# View logs
journalctl --user -u currents -f

# Restart service
systemctl --user restart currents
```

**3. Database Issues**
```bash
# Check database status
currents-history stats

# Clean old data
currents-history clean 365

# Recreate database
rm ~/.config/currents/weather_history.db
# Restart service to recreate
```

**4. Multi-Location Issues**
```bash
# Check orchestrator status
currents-orchestrator --status

# Check collector health
currents-orchestrator --health

# Restart failed collectors
currents-orchestrator --restart-failed
```

### Debug Mode

Run components in debug mode for detailed logging:

```bash
# Daemon with debug logging
RUST_LOG=debug currents --foreground

# Orchestrator with debug logging
RUST_LOG=debug currents-orchestrator config.toml

# Forecast with debug logging
RUST_LOG=debug currents-forecast
```

### Log Files

- **Daemon logs**: `journalctl --user -u currents -f`
- **Orchestrator logs**: Console output or log file
- **Database logs**: SQLite database at configured path

---

## Advanced Usage

### Custom Alert Conditions

Create complex alert conditions:

```toml
[[alerts]]
name = "severe-weather"
enabled = true
message = "Severe weather conditions detected!"
repeat = "once"
[alerts.condition.temperature]
min = 35.0
[alerts.condition.wind_speed]
min = 20.0
[alerts.condition.precipitation]
probability_min = 0.8
```

### API Rate Limit Management

Monitor and manage API usage:

```bash
# Check current usage
currents --api-stats

# View usage history
currents-history analyze api_usage 30d

# Configure limits
# Edit config.toml:
[weather]
api_daily_limit = 800  # Conservative limit
```

### Database Optimization

Optimize database performance:

```toml
[history]
enable_compression = true
compression_threshold = 30  # Compress data older than 30 days
max_history_days = 365      # Keep only last year
```

### Multi-Provider Setup

Use different providers for different locations:

```toml
[orchestrator.locations.london]
weather.provider = "openweathermap"
weather.api_key = "openweathermap-key"

[orchestrator.locations.tokyo]
weather.provider = "weatherapi"
weather.api_key = "weatherapi-key"
```

### Custom Collection Strategies

Configure different collection strategies per location:

```toml
[orchestrator.locations.denver]
collection_strategy = "threshold"  # Collect when conditions change
weather.collection_interval = 3600  # Base interval

[orchestrator.locations.london]
collection_strategy = "interval"    # Regular collection
weather.collection_interval = 1800  # Every 30 minutes
```

### Integration with Status Bars

Export weather data for status bars:

```bash
# Get current weather data
currents --output

# Get forecast data
currents-forecast --json

# Get historical data
currents-history export json 1d
```

### Systemd Service Management

```bash
# Enable service
sudo systemctl enable currents@$USER

# Start service
sudo systemctl start currents@$USER

# Stop service
sudo systemctl stop currents@$USER

# Restart service
sudo systemctl restart currents@$USER

# Check status
systemctl --user status currents

# View logs
journalctl --user -u currents -f
```

---

## Getting Help

### Documentation
- **README.md**: Overview and basic usage
- **USER_GUIDE.md**: This comprehensive guide
- **PHASE3_SUMMARY.md**: Multi-location features
- **nix/INSTALLATION.md**: NixOS installation guide

### Configuration Examples
- **config.toml**: Single location configuration
- **config.orchestrator.toml.example**: Multi-location configuration
- **nix/example-config.nix**: NixOS configuration

### Testing
- **demo-phase3.sh**: Multi-location demonstration
- **Integration tests**: `cargo test --package currents-orchestrator`

### Support
- Check logs for error messages
- Verify API keys and configuration
- Test with `--foreground` mode for debugging
- Use `--test-notification` to verify setup

---

**Currents** - A fast, lightweight weather monitoring system with multi-location support, built in Rust for reliability and performance.
