# Currents, a weather alert daemon

A fast, lightweight daemon written in Rust that monitors weather conditions and sends desktop notifications when specific weather patterns occur.

## Features

- **Fast and Efficient**: Built with Rust for excellent performance and memory safety
- **Flexible Configuration**: Supports both TOML and KDL formats for human-readable configuration
- **Multiple Weather Providers**: Supports OpenWeatherMap and WeatherAPI
- **Rich Alert Conditions**: Temperature, humidity, wind speed, precipitation, and description-based alerts
- **5-Day Forecast**: Beautiful, emoji-rich forecast display with detailed weather information
- **API Rate Limiting**: Automatic tracking to ensure you stay under 1000 calls/day (prevents charges)
- **System Integration**: systemd service with proper logging and resource limits
- **Desktop Notifications**: Uses libnotify for native Linux notifications

## Installation

### Option 1: Install from source (recommended)

1. **Install the binary**:
   ```bash
   cargo install --path .
   ```

2. **Install the systemd service** (installs but doesn't start):
   ```bash
   ./install-service.sh
   ```

3. **Configure the service**:
   - Copy `config.toml` to `~/.config/currents/config.toml` (or use `config.kdl.example` as a template for KDL format)
   - Update the configuration with your API key and preferences

4. **Start the service** (when ready):
   ```bash
   sudo systemctl enable currents@$USER
   sudo systemctl start currents@$USER
   ```

### Option 2: Manual installation

1. **Build the project**:
   ```bash
   cargo build --release
   ```

2. **Install the systemd service**:
   ```bash
   sudo cp currents.service /etc/systemd/system/
   sudo systemctl daemon-reload
   ```

3. **Configure and start** (same as Option 1, steps 3-4)

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

### Running in Foreground (for testing)
```bash
./target/release/currents --foreground
```

### Testing Notifications

You can test your notification setup with several options:

```bash
# Simple test (no config file required)
./target/release/currents --test-simple

# Test with your configuration
./target/release/currents --config config.toml --test-notification

# Custom test notification
./target/release/currents --config config.toml --test-custom "My Test" "This is a custom message"
```

### Weather Forecast

Display a beautiful 5-day forecast directly in your terminal:

```bash
# Show 5-day forecast
./target/release/currents --forecast

# With custom config path
./target/release/currents --config ~/.config/currents/config.toml --forecast
```

The forecast displays:
- Daily high and low temperatures
- Weather conditions with emoji icons
- Humidity, wind speed, and precipitation probability
- Automatically checks API rate limits before making requests

### API Usage Monitoring

Keep track of your API usage to stay under the free tier limit (1000 calls/day):

```bash
# Check current day's API call count
./target/release/currents --api-stats

# View cached weather data (no API call)
./target/release/currents --output
```

**Rate Limiting**: The daemon automatically prevents API calls if you've reached 1000 calls for the day (UTC). This ensures you never incur charges from weather providers. The limit resets at midnight UTC.

### Service Management
```bash
# Check status
sudo systemctl status currents@$USER

# View logs
sudo journalctl -u currents@$USER -f

# Restart service
sudo systemctl restart currents@$USER
```

## Development

The project is structured as follows:

- `src/main.rs` - Entry point and CLI handling
- `src/config.rs` - Configuration parsing with KDL support
- `src/weather.rs` - Weather data fetching from various APIs (current + forecast)
- `src/alerts.rs` - Alert condition matching engine
- `src/notifications.rs` - Desktop notification system
- `src/daemon.rs` - Main daemon logic and polling
- `src/api_stats.rs` - API call tracking and rate limiting
- `src/forecast_display.rs` - Pretty forecast formatting with emojis
- `currents.service` - systemd service template
- `install-service.sh` - Script to install systemd service (installs but doesn't start)

## Dependencies

- `tokio` - Async runtime
- `reqwest` - HTTP client for weather APIs (with rustls-tls)
- `toml` - TOML configuration format parsing
- `notify-rust` - Desktop notifications
- `tracing` - Structured logging
- `serde` - Serialization
- `anyhow` - Error handling
- `clap` - CLI argument parsing

## License

This project is licensed under the [Peer Production License](https://wiki.p2pfoundation.net/Peer_Production_License).

The Peer Production License is a copyfarleft license that allows:
- ✅ Use, modification, and distribution by worker-owned cooperatives
- ✅ Use by non-profit organizations and individuals
- ❌ Commercial use by traditional for-profit corporations that employ wage labor

This ensures the software remains a commons for those who contribute to the commons.
