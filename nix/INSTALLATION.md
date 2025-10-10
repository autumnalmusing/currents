# Nix Installation Guide for Currents

## Installation Options

Currents provides three separate packages you can install:

### Option 1: Just the Daemon (Recommended for most users)

```nix
{
  services.currents = {
    enable = true;
    # package defaults to currents-daemon
    
    weather = {
      apiKeyFile = "/run/secrets/openweathermap-api-key";
      location = "Denver,US";
      units = "metric";
      provider = "weatherapi";
    };
    
    alerts = [
      # Your alert rules
    ];
  };
}
```

**What you get:**
- `currents` binary (daemon for alerts)
- Lightweight systemd service
- Cache writing for Waybar

**What's NOT included:**
- Forecast display tool (install separately if needed)

### Option 2: Daemon + Forecast Tool

```nix
{
  services.currents = {
    enable = true;
    package = inputs.currents.packages.${pkgs.system}.currents-full;
    
    # ... rest of config
  };
  
  # Both binaries will be available:
  # - currents (daemon)
  # - currents-forecast
}
```

**What you get:**
- Both `currents` and `currents-forecast` binaries
- Systemd service for daemon
- Full feature set

### Option 3: Just Forecast Tool (No Daemon)

If you only want the forecast display without alerts:

```nix
{
  home.packages = [
    inputs.currents.packages.${pkgs.system}.currents-forecast
  ];
  
  # Create config file manually
  xdg.configFile."currents/config.toml".text = ''
    [weather]
    api_key = "your-key"
    location = "Denver,US"
    units = "metric"
    provider = "weatherapi"
  '';
}
```

**What you get:**
- Only `currents-forecast` binary
- No daemon, no systemd service
- Useful for checking weather on-demand

## Available Packages

| Package | Binary(ies) | Use Case |
|---------|------------|----------|
| `currents-daemon` | `currents` | Background alerts only (default) |
| `currents-forecast` | `currents-forecast` | Forecast display only |
| `currents-full` | `currents` + `currents-forecast` | Complete feature set |

## Package Selection Examples

### In home.nix:

```nix
# Just daemon (default)
services.currents.enable = true;

# OR daemon + forecast
services.currents = {
  enable = true;
  package = inputs.currents.packages.${pkgs.system}.currents-full;
};

# OR just forecast (no service)
home.packages = [ inputs.currents.packages.${pkgs.system}.currents-forecast ];
```

### In configuration.nix (system-wide):

```nix
# Install for all users
environment.systemPackages = [
  inputs.currents.packages.${pkgs.system}.currents-daemon
  inputs.currents.packages.${pkgs.system}.currents-forecast
];
```

## Flake Usage

```bash
# Run without installing
nix run github:autumnalmusing/currents#currents-daemon -- --help
nix run github:autumnalmusing/currents#currents-forecast

# Install to profile
nix profile install github:autumnalmusing/currents#currents-daemon
nix profile install github:autumnalmusing/currents#currents-forecast

# Try in shell
nix shell github:autumnalmusing/currents#currents-daemon
nix shell github:autumnalmusing/currents#currents-forecast
```

## Recommended Setup

**For typical usage (alerts + occasional forecast):**

```nix
{
  # Enable daemon with service
  services.currents = {
    enable = true;
    package = inputs.currents.packages.${pkgs.system}.currents-full;
    # ... config
  };
}
```

This gives you:
- ✅ Background daemon for alerts
- ✅ Systemd service management
- ✅ `currents-forecast` command available
- ✅ Single configuration file

**For minimal installations (alerts only):**

```nix
{
  services.currents = {
    enable = true;
    # Uses default package (currents-daemon)
    # ... config
  };
}
```

This gives you:
- ✅ Background daemon for alerts
- ✅ Systemd service
- ❌ No forecast tool (smaller closure size)

You can always add forecast later by changing to `currents-full`.

## Checking What's Installed

```bash
# See which currents binaries are available
which currents
which currents-forecast

# Check package info
nix path-info --json /run/current-system/sw/bin/currents
```

