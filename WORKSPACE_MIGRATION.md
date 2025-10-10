# Workspace Migration Guide

## What Changed

Currents has been refactored from a monolithic crate into a Cargo workspace with three independent crates:

### Before:
```
currents/
├── src/           # Everything in one crate
└── Cargo.toml     # Single package
```

### After:
```
currents/
├── Cargo.toml              # Workspace definition
├── currents-core/          # Shared library
├── currents-daemon/        # Alert daemon
└── currents-forecast/      # Forecast tool (new separate binary!)
```

## Breaking Changes

### Command Line

**Old:**
```bash
currents --forecast        # Built into daemon
```

**New:**
```bash
currents-forecast          # Separate binary
```

The `--forecast` flag on the daemon now shows an error message directing users to install `currents-forecast`.

### Installation

**Old:**
```bash
cargo install --path .
# Installed one binary: currents
```

**New:**
```bash
# Install both tools
cargo install --path currents-daemon
cargo install --path currents-forecast

# Or install workspace (both)
cargo install --path . --workspace

# Or install just what you need
cargo install --path currents-daemon     # Just alerts
cargo install --path currents-forecast   # Just forecast
```

### Configuration

**No changes!** The same `~/.config/currents/config.toml` file works for all tools.

Each tool reads only the sections it needs:
- `currents` (daemon) reads: `[weather]`, `[alerts]`, `[notifications]`, `[polling]`, `[cache]`
- `currents-forecast` reads: `[weather]`, `[forecast_display]`, `[forecast_highlights]`

## Benefits

### For Users

✅ **Smaller binaries** - Only install what you need  
✅ **Faster startup** - Daemon is lighter without forecast code  
✅ **Independent tools** - Use forecast without running daemon  
✅ **Same config file** - No configuration changes needed

### For Developers

✅ **Better separation** - Each crate has focused purpose  
✅ **Easier testing** - Test components independently  
✅ **Code reuse** - Shared types via `currents-core`  
✅ **Future extensibility** - Easy to add `currents-history`, `currents-widget`, etc.

## Migration Steps

### If you already have currents installed:

1. **Uninstall old version:**
   ```bash
   cargo uninstall currents
   ```

2. **Install new workspace version:**
   ```bash
   cd currents
   git pull  # Get latest workspace structure
   cargo install --path currents-daemon
   cargo install --path currents-forecast  # Optional
   ```

3. **Update any scripts:**
   - Replace `currents --forecast` with `currents-forecast`
   - Everything else works the same

4. **Restart daemon:**
   ```bash
   systemctl --user restart currents
   ```

### Config file compatibility

Your existing config file works without changes! The workspace preserves full backward compatibility.

## What's Next

The workspace architecture makes it easy to add new tools:

- **`currents-history`** - Track weather trends over time
- **`currents-widget`** - Desktop widget for current conditions
- **`currents-alert-editor`** - GUI for editing alert rules
- **`currents-export`** - Export weather data in various formats

All will share the same configuration file and core library.

