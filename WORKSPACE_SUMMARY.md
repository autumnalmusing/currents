# Currents Workspace - Quick Reference

## Project Structure

```
currents/                           # Single Git repository
├── Cargo.toml                      # Workspace root
├── config.toml                     # Shared configuration file
│
├── currents-core/                  # Library crate (511 lines)
│   └── Provides: API clients, data types, rate limiting
│
├── currents-daemon/                # Daemon binary (~800 lines)
│   └── Binary: currents
│   └── Purpose: Background weather monitoring + alerts
│
└── currents-forecast/              # Forecast binary (~450 lines)
    └── Binary: currents-forecast
    └── Purpose: Display 5-day weather forecast
```

## Quick Commands

### Installation
```bash
# Install daemon only
cargo install --path currents-daemon

# Install forecast only  
cargo install --path currents-forecast

# Install both
cargo install --path currents-daemon
cargo install --path currents-forecast
```

### Development
```bash
# Build everything
cargo build --workspace

# Test everything
cargo test --workspace

# Build specific crate
cargo build -p currents-daemon
cargo build -p currents-forecast
cargo build -p currents-core

# Run binaries
cargo run -p currents-daemon -- --foreground
cargo run -p currents-forecast
```

### Usage
```bash
# Daemon
currents --foreground              # Run in foreground
currents --test-simple             # Test notifications
currents --api-stats               # Check API usage
currents --output                  # View cached data

# Forecast
currents-forecast                  # Show 5-day forecast
currents-forecast --days 3         # Show 3-day forecast
```

## Configuration

Single file (`~/.config/currents/config.toml`) is shared by all tools:

```toml
# Core - Used by all tools
[weather]
api_key = "..."
location = "London,UK"
provider = "weatherapi"

# Daemon-specific
[notifications]
urgency = "normal"

[polling]
interval_seconds = 300

[[alerts]]
name = "hot-weather"
enabled = true
message = "It's hot!"

# Forecast-specific
[forecast_display]
show_pressure = true
show_uv = true
show_aqi = true

[forecast_highlights.temperature]
high = 30.0
high_color = "red"
```

## Test Summary

- **currents-core**: 0 tests (library types)
- **currents-daemon**: 10 tests (config, alerts, api_stats)
- **currents-forecast**: 8 tests (display, formatting, colors)
- **Total**: 20 tests passing ✅

## Data Fields Available

### Always Captured:
- Temperature (min/max)
- Humidity
- Wind speed
- Precipitation
- Description

### Optional (configurable):
- **Pressure** - Both APIs
- **Cloud cover** - Both APIs
- **Wind direction** - Both APIs
- **Wind gusts** - Both APIs
- **Visibility** - WeatherAPI only
- **UV index** - WeatherAPI only
- **AQI** - WeatherAPI only

## Next Steps

1. **Implement Enhanced Alerts** - Use new data fields in alert rules
2. **Add currents-history** - Historical weather tracking
3. **Clean up old src/** - Remove legacy code directory

## Notes

- Old `src/` directory preserved for reference
- `Cargo.toml.old` backed up
- All functionality preserved
- Zero regressions
- Backward compatible config

