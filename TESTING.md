# Testing Guide for Currents

This document describes the testing infrastructure for the currents weather daemon.

## Running Tests

### All Tests
```bash
cargo test
```

### Specific Test Suite
```bash
cargo test --test config_test
cargo test --test forecast_display_test
cargo test --test api_stats_test
```

### With Output
```bash
cargo test -- --nocapture
```

### Using Just
```bash
just test           # Run all tests
just test-verbose   # Run with output
just test-one NAME  # Run specific test
```

## Test Coverage

### Configuration Tests (`tests/config_test.rs`)

Tests for configuration loading and validation:

- **`test_load_toml_config`** - Verifies TOML config parsing with all fields
- **`test_default_cache_config`** - Tests default cache configuration values
- **`test_default_forecast_highlights`** - Tests default highlight color configuration
- **`test_alert_repeat_default`** - Verifies `repeat = "once"` is the default for alerts
- **`test_custom_forecast_colors`** - Tests custom hex colors and highlight thresholds

### Forecast Display Tests (`tests/forecast_display_test.rs`)

Tests for the forecast display formatting:

- **`test_forecast_format_basic`** - Basic forecast formatting output
- **`test_forecast_with_temperature_highlights`** - Temperature threshold highlighting
- **`test_forecast_with_custom_colors`** - Named color customization
- **`test_forecast_with_hex_colors`** - Hex color code support (#ff5733 format)
- **`test_forecast_all_highlights_enabled`** - All highlight types simultaneously
- **`test_forecast_no_highlights`** - Formatting without any highlights

### API Stats Tests (`tests/api_stats_test.rs`)

Tests for API call tracking:

- **`test_api_stats_tracker_initialization`** - Tracker initialization and file creation
- **`test_api_stats_increment`** - Incrementing call count
- **`test_api_stats_remaining_calls`** - Calculating remaining daily calls
- **`test_api_stats_at_limit`** - Behavior at daily limit
- **`test_api_stats_persistence`** - State persistence across restarts
- **`test_api_stats_day_rollover`** - Daily reset at midnight UTC

## Nix Module Verification

The Nix home-manager module includes built-in type checking and validation:

- Type checking for all options (automatic with Nix)
- Validation that either `apiKey` or `apiKeyFile` is set (build-time assertion)
- Mutual exclusion checks between `apiKey` and `apiKeyFile` (build-time assertion)
- TOML config generation from Nix options

### Verify the Flake

```bash
nix flake check
# or
just nix-verify
```

### Test in Your Config

Add to your home-manager configuration:

```nix
services.currents = {
  enable = true;
  weather.apiKey = "test-key";
  weather.location = "Test City";
};
```

Rebuild and verify:
```bash
# Rebuild
home-manager switch --flake .#yourconfig

# Check generated config
cat ~/.config/currents/config.toml

# Check service status
systemctl --user status currents

# View logs
journalctl --user -u currents -f
```

The Nix module's type system catches configuration errors at build time, before deployment.

## Test Data

Tests use:
- `tempfile` crate for temporary directories and files
- Mock weather data with various conditions (high/low temps, humidity, wind, etc.)
- Configurable thresholds to test highlight logic

## Continuous Integration

For CI/CD pipelines, run:

```bash
# Full test suite with formatting and linting
just check
```

This runs:
1. `cargo test` - All unit and integration tests
2. `cargo clippy` - Linter checks
3. `cargo fmt --check` - Format verification

## Adding New Tests

### Rust Tests

1. Create a new file in `tests/` directory
2. Add test functions with `#[test]` attribute
3. Use `assert!`, `assert_eq!`, etc. for assertions
4. Run with `cargo test`

Example:
```rust
#[test]
fn test_my_feature() {
    let result = my_function();
    assert_eq!(result, expected_value);
}
```

### Nix Module Tests

1. Add a new test case to `nix/tests.nix`
2. Use `runTest` for full module tests
3. Use `lib.runTests` for simple assertion tests
4. Run with `nix-build nix/tests.nix`

Example:
```nix
my-test = runTest {
  name = "my-test-name";
  
  modules = [
    currentsModule
    {
      services.currents = {
        # ... config
      };
    }
  ];
  
  test = { config, ... }: {
    assertions = [
      {
        assertion = condition;
        message = "error message";
      }
    ];
  };
};
```

## Test Philosophy

- **Unit tests**: Test individual functions and modules in isolation
- **Integration tests**: Test how components work together
- **Configuration tests**: Verify configuration parsing and validation
- **Module tests**: Ensure Nix module generates correct configuration

## Dependencies

Test dependencies (in `Cargo.toml`):
```toml
[dev-dependencies]
tempfile = "3.14"
```

The `tempfile` crate provides temporary files and directories that are automatically cleaned up after tests.

