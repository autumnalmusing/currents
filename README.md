# Currents, a weather alert daemon

A fast, lightweight daemon written in Rust that monitors weather conditions and sends desktop notifications when specific weather patterns occur.

## Features

- **Fast and Efficient**: Built with Rust for excellent performance and memory safety
- **Flexible Configuration**: Supports both TOML and KDL formats for human-readable configuration
- **Multiple Weather Providers**: Supports OpenWeatherMap and WeatherAPI
- **Rich Alert Conditions**: Temperature, humidity, wind speed, precipitation, and description-based alerts
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

- **OpenWeatherMap**: Free tier available, requires API key
- **WeatherAPI**: Free tier available, requires API key

### Alert Conditions

- **Temperature**: Min/max temperature thresholds
- **Humidity**: Humidity level ranges
- **Wind Speed**: Wind speed thresholds
- **Precipitation**: Intensity and probability
- **Description**: Keyword matching on weather descriptions

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
- `src/weather.rs` - Weather data fetching from various APIs
- `src/alerts.rs` - Alert condition matching engine
- `src/notifications.rs` - Desktop notification system
- `src/daemon.rs` - Main daemon logic and polling
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

MIT License
