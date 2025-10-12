# Contributing to Currents

This guide is for developers who want to contribute to the Currents weather monitoring system.

## Development Setup

### Prerequisites
- Rust 1.75+ 
- Cargo
- Git

### Getting Started
```bash
git clone https://github.com/autumnalmusing/currents.git
cd currents
cargo build --workspace
```

## Workspace Structure

Currents is built as a Cargo workspace with independent, composable crates:

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
├── currents-history/             # Weather history and analysis tool
│   ├── src/
│   │   ├── main.rs              # History binary entry point
│   │   ├── lib.rs               # Library for tests
│   │   ├── database.rs          # SQLite database management
│   │   ├── analysis.rs          # Pattern analysis and trends
│   │   ├── config.rs            # History-specific config
│   │   ├── cli.rs               # CLI interface
│   │   └── visualization.rs     # ASCII chart rendering
│   ├── tests/
│   └── Cargo.toml
│
└── config.toml                   # Shared config file (all tools read this)
```

## Building and Testing

### Building
```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p currents-daemon
cargo build -p currents-forecast
cargo build -p currents-history
cargo build -p currents-core
```

### Testing
```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p currents-daemon
cargo test -p currents-forecast
cargo test -p currents-history
```

### Running Development Versions
```bash
# Run specific binary
cargo run -p currents-daemon -- --foreground
cargo run -p currents-forecast -- --days 3
cargo run -p currents-history -- history 7d
```

## Adding New Tools

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

## Development Workflow

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/your-feature-name`
3. **Make your changes**
4. **Run tests**: `cargo test --workspace`
5. **Build everything**: `cargo build --workspace`
6. **Commit your changes**: `git commit -m "Add your feature"`
7. **Push to your fork**: `git push origin feature/your-feature-name`
8. **Create a pull request**

## Code Style

- Follow Rust standard formatting: `cargo fmt`
- Run clippy for linting: `cargo clippy --workspace`
- Write tests for new functionality
- Update documentation for user-facing changes

## Testing

- Unit tests should be in each crate's `tests/` directory
- Integration tests should test the full workflow
- Test with both OpenWeatherMap and WeatherAPI providers
- Test error conditions and edge cases

## Documentation

- Update README.md for user-facing changes
- Update USER_GUIDE.md for new features
- Update DEPLOYMENT_GUIDE.md for deployment changes
- Add doc comments to public APIs
- Update this CONTRIBUTING.md for workflow changes
