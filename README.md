# Currents - Modular Weather Monitoring System

A fast, lightweight weather monitoring system written in Rust with a modular plugin architecture. The core daemon monitors weather conditions and sends desktop notifications based on custom alert rules.

## Architecture

Currents is built as a Cargo workspace with independent, composable crates:

- **`currents-core`** - Shared library with API clients and data types
- **`currents-daemon`** - Lightweight background daemon for weather alerts
- **`currents-forecast`** - Standalone forecast display tool
- **`currents-history`** (planned) - Historical weather tracking and analysis

Each tool can be installed independently based on your needs.

## Features

### Core Daemon (`currents`)
- **Lightweight Monitoring**: Efficient background process with minimal resource usage
- **Weather Alerts**: Custom notification rules based on weather conditions
- **API Rate Limiting**: Automatic tracking to stay under free tier limits (prevents charges)
- **Cache for Waybar**: Exports weather data for status bar integration
- **Multiple Weather Providers**: Supports OpenWeatherMap and WeatherAPI

### Forecast Tool (`currents-forecast`)
- **5-Day Weather Forecast**: Beautiful formatted table with comprehensive data
- **Configurable Display**: Show/hide columns (pressure, UV, AQI, visibility, etc.)
- **Color Highlighting**: Threshold-based highlighting with custom colors
- **Independent**: Runs standalone, doesn't require daemon

### Shared Features
- **Flexible Configuration**: Single TOML config file shared across all tools
- **Rich Data**: Temperature, humidity, wind, pressure, UV, AQI, cloud cover, and more
- **Provider Support**: OpenWeatherMap and WeatherAPI with automatic field mapping

## Installation

### Option 1: NixOS / Home Manager (Recommended for Nix users)

If you're using NixOS or home-manager, you can configure currents declaratively:

1. **Add to your flake inputs**:
   ```nix
   inputs.currents.url = "github:yourusername/currents";
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
```

**Install only what you need:**
```bash
# Just the daemon (minimal, alerts only)
cargo install --path currents-daemon

# Just the forecast tool (no daemon)
cargo install --path currents-forecast
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

The daemon supports both TOML and KDL configuration formats. The format is automatically detected based on the file extension:

- **TOML format** (`.toml`): Currently fully supported - see `config.toml` for an example
- **KDL format** (`.kdl`): Planned for future implementation - see `config.kdl.example` for the intended format

### Configuration Files
- `config.toml` - Working TOML configuration example
- `config.kdl.example` - Planned KDL configuration format (not yet implemented)

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

## Development

### Workspace Structure

```
currents/
├── Cargo.toml                    # Workspace definition
├── currents-core/                # Shared library
│   ├── src/
│   │   ├── lib.rs               # Public API
│   │   ├── types.rs             # WeatherData, ForecastDay, etc.
│   │   ├── api.rs               # API clients (OpenWeatherMap, WeatherAPI)
│   │   ├── api_stats.rs         # Rate limiting
│   │   └── config.rs            # Shared config types
│   └── Cargo.toml
│
├── currents-daemon/              # Alert daemon
│   ├── src/
│   │   ├── main.rs              # Daemon binary entry point
│   │   ├── lib.rs               # Library for tests
│   │   ├── config.rs            # Daemon-specific config
│   │   ├── alerts.rs            # Alert engine
│   │   ├── notifications.rs     # Desktop notifications
│   │   └── daemon.rs            # Polling loop
│   ├── tests/
│   └── Cargo.toml
│
├── currents-forecast/            # Forecast display tool
│   ├── src/
│   │   ├── main.rs              # Forecast binary entry point
│   │   ├── lib.rs               # Library for tests
│   │   ├── config.rs            # Forecast-specific config
│   │   └── display.rs           # Table formatting & colors
│   ├── tests/
│   └── Cargo.toml
│
└── config.toml                   # Shared config file (all tools read this)
```

### Building

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p currents-daemon
cargo build -p currents-forecast
cargo build -p currents-core

# Run tests
cargo test --workspace
cargo test -p currents-daemon
cargo test -p currents-forecast

# Run specific binary
cargo run -p currents-daemon -- --foreground
cargo run -p currents-forecast -- --days 3
```

### Adding New Tools

To add a new tool (e.g., `currents-history`):

1. Add to workspace members in root `Cargo.toml`
2. Create `currents-history/` directory with its own `Cargo.toml`
3. Add `currents-core` as dependency
4. Implement tool-specific config structures
5. Read from shared `~/.config/currents/config.toml`

All tools remain fully independent while sharing core types and API clients.

## Dependencies

### Core Library
- `reqwest` - HTTP client with rustls
- `serde` / `serde_json` - Serialization
- `chrono` - Date/time handling
- `tokio` - Async runtime
- `anyhow` - Error handling

### Daemon-Specific
- `notify-rust` - Desktop notifications
- `tracing` / `tracing-subscriber` - Logging

### Forecast-Specific
- `tabled` - Beautiful terminal tables
- `clap` - CLI arguments

## License

This project is licensed under the [Peer Production License](https://wiki.p2pfoundation.net/Peer_Production_License).

The Peer Production License is a copyfarleft license that allows:
- ✅ Use, modification, and distribution by worker-owned cooperatives
- ✅ Use by non-profit organizations and individuals
- ❌ Commercial use by traditional for-profit corporations that employ wage labor

This ensures the software remains a commons for those who contribute to the commons.
