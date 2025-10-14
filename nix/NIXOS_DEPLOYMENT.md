# Currents NixOS Deployment Guide

This guide shows you how to deploy Currents weather monitoring on a NixOS server using `virtualisation.oci-containers.containers`.

## 🚀 Quick Start

### 1. Add to Your NixOS Configuration

Add this to your `configuration.nix`:

```nix
{ config, lib, pkgs, ... }:

{
  # Import the Currents module
  imports = [
    ./currents-containers.nix
  ];

  # Enable the Currents service
  services.currents = {
    enable = true;
    weatherapiKey = "your-weatherapi-key-here";
    
    # Optional: Add more locations
    locations = {
      london = {
        name = "London, UK";
        coordinates = [51.5074 (-0.1278)];
        units = "metric";
        collectionInterval = 1800;
        strategy = "interval";
      };
      tokyo = {
        name = "Tokyo, JP";
        coordinates = [35.6762 139.6503];
        units = "metric";
        collectionInterval = 1800;
        strategy = "interval";
      };
    };
  };
}
```

### 2. Build and Deploy

```bash
# Build the configuration
sudo nixos-rebuild build

# Apply the configuration
sudo nixos-rebuild switch
```

### 3. Check Status

```bash
# Check if the container is running
sudo systemctl status container@currents-orchestrator

# View logs
sudo journalctl -u container@currents-orchestrator -f

# Use the management script
manage-currents status
```

## 🔧 Configuration Options

### Basic Configuration

```nix
services.currents = {
  enable = true;
  weatherapiKey = "your-api-key";
  
  # Database path
  databasePath = "/var/lib/currents/orchestrator.db";
  
  # Collection settings
  analysisInterval = 3600;  # 1 hour
  healthCheckInterval = 300;  # 5 minutes
};
```

### Location Configuration

```nix
services.currents.locations = {
  london = {
    name = "London, UK";
    coordinates = [51.5074 (-0.1278)];
    units = "metric";
    collectionInterval = 1800;  # 30 minutes
    strategy = "interval";
  };
  
  tokyo = {
    name = "Tokyo, JP";
    coordinates = [35.6762 139.6503];
    units = "metric";
    collectionInterval = 1800;
    strategy = "interval";
  };
  
  denver = {
    name = "Denver, CO, US";
    coordinates = [39.7392 (-104.9903)];
    units = "imperial";
    collectionInterval = 1800;
    strategy = "interval";
  };
};
```

### Collection Strategies

- **`interval`**: Collect at regular intervals
- **`threshold`**: Collect when conditions change significantly
- **`hybrid`**: Combination of both

## 🎯 What You Get

### Services

- **currents-orchestrator**: Main data collection service
- **currents-forecast**: Weather forecast display
- **currents-history**: Historical data analysis

### Multi-Location Architecture

The system uses an efficient multi-location collector architecture:

- **5-10 locations per collector process** (configurable)
- **Automatic load balancing** across collector groups
- **Fault isolation** - if one collector fails, others continue
- **Resource efficient** - much better than 1 location per process
- **Scalable** - designed for 50+ locations

**Example with 50 locations:**
- **10 collector processes** (5 locations each)
- **1 orchestrator container** (main coordinator)
- **1 shared database** (SQLite)
- **Total: 11 processes** instead of 51 processes

### Data Collection

- **Locations**: London, UK and Tokyo, JP (configurable)
- **Frequency**: Every 30 minutes (configurable)
- **Storage**: Centralized SQLite database
- **Retention**: 1 year (configurable)
- **Provider**: WeatherAPI (supports historical data up to 365 days)
- **Architecture**: Multi-location collectors (5-10 locations per process)
- **Scalability**: Optimized for 50+ locations with efficient resource usage

### Management Commands

```bash
# Start services
manage-currents start

# Stop services
manage-currents stop

# View logs
manage-currents logs

# Show forecast
manage-currents forecast

# Show history
manage-currents history

# Check status
manage-currents status

# Backup database
manage-currents backup
```

## 📊 Monitoring

### Check Status

```bash
# System service status
sudo systemctl status container@currents-orchestrator

# Container status
sudo podman ps

# View logs
sudo journalctl -u container@currents-orchestrator -f
```

### View Weather Data

```bash
# Show forecast
manage-currents forecast

# Show history
manage-currents history

# Show database stats
sudo podman exec currents-orchestrator currents-history stats
```

## 💾 Backup

### Manual Backup

```bash
manage-currents backup
```

### Automated Backup

Add to your NixOS configuration:

```nix
# Enable automatic backups
services.borgbackup = {
  enable = true;
  jobs = {
    currents = {
      paths = [ "/var/lib/currents" ];
      repo = "user@backup-server:/backup/currents";
      encryption = {
        mode = "repokey";
      };
      compression = "auto,zstd";
      startAt = "daily";
    };
  };
};
```

## 🔧 Troubleshooting

### Common Issues

1. **Container won't start:**
   ```bash
   sudo journalctl -u container@currents-orchestrator -f
   ```

2. **API key issues:**
   - Check your WeatherAPI key in the configuration
   - Verify API key is valid at https://weatherapi.com
   - Check API rate limits (1000 calls/day free tier)

3. **Database issues:**
   ```bash
   manage-currents status
   ```

4. **Permission issues:**
   ```bash
   sudo chown -R currents:currents /var/lib/currents
   ```

### Reset System

```bash
# Stop the container
sudo systemctl stop container@currents-orchestrator

# Remove the container
sudo podman rm currents-orchestrator

# Remove data (WARNING: This will delete all data)
sudo rm -rf /var/lib/currents/*

# Rebuild and restart
sudo nixos-rebuild switch
```

## 📈 Scaling

### Add More Locations

Edit your NixOS configuration:

```nix
services.currents.locations = {
  # ... existing locations ...
  
  new_york = {
    name = "New York, US";
    coordinates = [40.7128 (-74.0060)];
    units = "imperial";
    collectionInterval = 1800;
    strategy = "interval";
  };
};
```

### Adjust Collection Frequency

```nix
services.currents.locations.london = {
  # ... other settings ...
  collectionInterval = 3600;  # 1 hour instead of 30 minutes
};
```

### Change Collection Strategy

```nix
services.currents.locations.london = {
  # ... other settings ...
  strategy = "threshold";  # Collect when conditions change
};
```

## 🎯 Production Tips

1. **Use secrets for API keys:**
   ```nix
   services.currents.weatherapiKey = "/run/secrets/weatherapi-key";
   ```

2. **Enable log rotation:**
   ```nix
   services.logrotate = {
     enable = true;
     rules = {
       currents = {
         files = [ "/var/lib/currents/logs/*.log" ];
         frequency = "daily";
         rotate = 30;
         compress = true;
       };
     };
   };
   ```

3. **Monitor disk space:**
   ```nix
   services.prometheus.exporters.node = {
     enable = true;
     enabledCollectors = [ "filesystem" ];
   };
   ```

4. **Set up alerts:**
   ```nix
   services.prometheus.alertmanager = {
     enable = true;
     configuration = {
       route = {
         group_by = [ "alertname" ];
         group_wait = "10s";
         group_interval = "10s";
         repeat_interval = "1h";
         receiver = "web.hook";
       };
       receivers = [ { name = "web.hook"; } ];
     };
   };
   ```

## 🎉 Success!

Your Currents weather monitoring system is now running on your NixOS server!

- **Data Collection**: Automatic every 30 minutes
- **Multiple Locations**: London and Tokyo (configurable)
- **Centralized Database**: All data in one place
- **Easy Management**: Simple commands for everything
- **Declarative Configuration**: Everything managed by NixOS

The system will start collecting weather data immediately and build up a historical database over time.
