# Currents Deployment Guide

**Production deployment guide for the Currents weather monitoring system**

## Table of Contents

1. [Production Architecture](#production-architecture)
2. [Docker Deployment](#docker-deployment)
3. [Systemd Services](#systemd-services)
4. [Monitoring and Logging](#monitoring-and-logging)
5. [Security Considerations](#security-considerations)
6. [Backup and Recovery](#backup-and-recovery)
7. [Scaling Considerations](#scaling-considerations)
8. [Troubleshooting](#troubleshooting)

---

## Production Architecture

### Single Location Deployment

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Systemd       │    │   Currents      │    │   Weather APIs  │
│   Service       │───▶│   Daemon        │───▶│   (OpenWeather, │
│   (currents)    │    │   (Background)  │    │    WeatherAPI)  │
└─────────────────┘    └─────────┬───────┘    └─────────────────┘
                                │
                                ▼
                        ┌─────────────────┐
                        │   SQLite DB     │
                        │   (Local)       │
                        └─────────────────┘
```

### Multi-Location Deployment

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Orchestrator  │    │   Collector 1   │    │   Collector 2   │
│   (Manager)     │───▶│   (London)      │    │   (Tokyo)       │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                    ┌─────────────▼─────────────┐
                    │   Centralized Storage     │
                    │   (SQLite Database)       │
                    └────────────────────────────┘
```

---

## Docker Deployment

### Single Location Container

**Dockerfile:**
```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --package currents-daemon

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/currents /usr/local/bin/
COPY config.toml /etc/currents/config.toml

RUN mkdir -p /var/lib/currents
VOLUME ["/var/lib/currents"]

EXPOSE 8080

CMD ["currents", "--foreground"]
```

**docker-compose.yml:**
```yaml
version: '3.8'

services:
  currents:
    build: .
    container_name: currents-daemon
    restart: unless-stopped
    volumes:
      - ./config.toml:/etc/currents/config.toml:ro
      - currents-data:/var/lib/currents
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "currents", "--api-stats"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  currents-data:
```

### Multi-Location Container

**Dockerfile.orchestrator:**
```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release --package currents-orchestrator

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/currents-orchestrator /usr/local/bin/
COPY config.orchestrator.toml /etc/currents/orchestrator.toml

RUN mkdir -p /var/lib/currents
VOLUME ["/var/lib/currents"]

EXPOSE 8080

CMD ["currents-orchestrator", "/etc/currents/orchestrator.toml"]
```

**docker-compose.orchestrator.yml:**
```yaml
version: '3.8'

services:
  currents-orchestrator:
    build:
      context: .
      dockerfile: Dockerfile.orchestrator
    container_name: currents-orchestrator
    restart: unless-stopped
    volumes:
      - ./config.orchestrator.toml:/etc/currents/orchestrator.toml:ro
      - currents-data:/var/lib/currents
    environment:
      - RUST_LOG=info
    healthcheck:
      test: ["CMD", "currents-orchestrator", "--health"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  currents-data:
```

### Deployment Commands

```bash
# Build and start single location
docker-compose up -d

# Build and start multi-location
docker-compose -f docker-compose.orchestrator.yml up -d

# View logs
docker-compose logs -f

# Stop services
docker-compose down

# Update services
docker-compose pull
docker-compose up -d
```

---

## Systemd Services

### Single Location Service

**/etc/systemd/system/currents@.service:**
```ini
[Unit]
Description=Currents Weather Daemon for %i
After=network.target
Wants=network.target

[Service]
Type=simple
User=%i
Group=%i
WorkingDirectory=/home/%i/.config/currents
ExecStart=/usr/local/bin/currents --foreground
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/%i/.config/currents

# Resource limits
MemoryLimit=256M
CPUQuota=50%

[Install]
WantedBy=multi-user.target
```

### Multi-Location Service

**/etc/systemd/system/currents-orchestrator@.service:**
```ini
[Unit]
Description=Currents Orchestrator for %i
After=network.target
Wants=network.target

[Service]
Type=simple
User=%i
Group=%i
WorkingDirectory=/home/%i/.config/currents
ExecStart=/usr/local/bin/currents-orchestrator /home/%i/.config/currents/orchestrator.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/%i/.config/currents

# Resource limits
MemoryLimit=512M
CPUQuota=100%

[Install]
WantedBy=multi-user.target
```

### Service Management

```bash
# Enable and start single location service
sudo systemctl enable currents@$USER
sudo systemctl start currents@$USER

# Enable and start multi-location service
sudo systemctl enable currents-orchestrator@$USER
sudo systemctl start currents-orchestrator@$USER

# Check status
systemctl status currents@$USER
systemctl status currents-orchestrator@$USER

# View logs
journalctl -u currents@$USER -f
journalctl -u currents-orchestrator@$USER -f

# Restart services
sudo systemctl restart currents@$USER
sudo systemctl restart currents-orchestrator@$USER
```

---

## Monitoring and Logging

### Log Configuration

**Structured logging with different levels:**

```toml
# In your config file
[logging]
level = "info"  # debug, info, warn, error
format = "json"  # json, pretty
```

**Environment variables:**
```bash
# Set log level
export RUST_LOG=info

# Set specific component logging
export RUST_LOG=currents=debug,currents_core=info

# JSON logging for production
export RUST_LOG_FORMAT=json
```

### Health Checks

**Single location health check:**
```bash
#!/bin/bash
# /usr/local/bin/currents-health-check.sh

# Check if daemon is running
if ! systemctl is-active --quiet currents@$USER; then
    echo "Currents daemon is not running"
    exit 1
fi

# Check API connectivity
if ! currents --api-stats > /dev/null 2>&1; then
    echo "API connectivity check failed"
    exit 1
fi

# Check database
if ! currents-history stats > /dev/null 2>&1; then
    echo "Database check failed"
    exit 1
fi

echo "All health checks passed"
exit 0
```

**Multi-location health check:**
```bash
#!/bin/bash
# /usr/local/bin/currents-orchestrator-health-check.sh

# Check if orchestrator is running
if ! systemctl is-active --quiet currents-orchestrator@$USER; then
    echo "Currents orchestrator is not running"
    exit 1
fi

# Check orchestrator health
if ! currents-orchestrator --health > /dev/null 2>&1; then
    echo "Orchestrator health check failed"
    exit 1
fi

# Check collector processes
if ! currents-orchestrator --processes > /dev/null 2>&1; then
    echo "Collector process check failed"
    exit 1
fi

echo "All health checks passed"
exit 0
```

### Monitoring Integration

**Prometheus metrics (custom implementation):**
```rust
// Example metrics endpoint
use prometheus::{Counter, Gauge, Registry};

pub struct CurrentsMetrics {
    pub api_calls_total: Counter,
    pub weather_temperature: Gauge,
    pub weather_humidity: Gauge,
    pub alerts_triggered_total: Counter,
}

impl CurrentsMetrics {
    pub fn new() -> Self {
        Self {
            api_calls_total: Counter::new("currents_api_calls_total", "Total API calls made").unwrap(),
            weather_temperature: Gauge::new("currents_weather_temperature", "Current temperature").unwrap(),
            weather_humidity: Gauge::new("currents_weather_humidity", "Current humidity").unwrap(),
            alerts_triggered_total: Counter::new("currents_alerts_triggered_total", "Total alerts triggered").unwrap(),
        }
    }
}
```

**Grafana dashboard configuration:**
```json
{
  "dashboard": {
    "title": "Currents Weather Monitoring",
    "panels": [
      {
        "title": "API Calls",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(currents_api_calls_total[5m])",
            "legendFormat": "API Calls/sec"
          }
        ]
      },
      {
        "title": "Temperature",
        "type": "graph",
        "targets": [
          {
            "expr": "currents_weather_temperature",
            "legendFormat": "Temperature"
          }
        ]
      }
    ]
  }
}
```

---

## Security Considerations

### API Key Management

**Use environment variables:**
```bash
# Set API key as environment variable
export OPENWEATHER_API_KEY="your-api-key"

# Use in configuration
[weather]
api_key = "${OPENWEATHER_API_KEY}"
```

**Use secret files:**
```bash
# Create secret file
echo "your-api-key" > /run/secrets/weatherapi-key
chmod 600 /run/secrets/weatherapi-key

# Reference in configuration
[weather]
api_key_file = "/run/secrets/weatherapi-key"
```

### File Permissions

```bash
# Secure configuration files
chmod 600 ~/.config/currents/config.toml
chmod 600 ~/.config/currents/orchestrator.toml

# Secure database files
chmod 600 ~/.config/currents/weather_history.db
chmod 600 ~/.config/currents/orchestrator.db

# Secure log files
chmod 644 /var/log/currents/
```

### Network Security

**Firewall rules:**
```bash
# Allow only outbound HTTPS (for API calls)
iptables -A OUTPUT -p tcp --dport 443 -j ACCEPT
iptables -A OUTPUT -p tcp --dport 80 -j ACCEPT

# Block inbound connections (if running web interface)
iptables -A INPUT -p tcp --dport 8080 -j DROP
```

**API rate limiting:**
```toml
[weather]
api_daily_limit = 800  # Conservative limit
api_rate_limit = 60    # Calls per minute
```

---

## Backup and Recovery

### Database Backup

**Automated backup script:**
```bash
#!/bin/bash
# /usr/local/bin/currents-backup.sh

BACKUP_DIR="/var/backups/currents"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup single location database
if [ -f ~/.config/currents/weather_history.db ]; then
    cp ~/.config/currents/weather_history.db $BACKUP_DIR/weather_history_$DATE.db
fi

# Backup orchestrator database
if [ -f ~/.config/currents/orchestrator.db ]; then
    cp ~/.config/currents/orchestrator.db $BACKUP_DIR/orchestrator_$DATE.db
fi

# Compress backups
gzip $BACKUP_DIR/*_$DATE.db

# Keep only last 30 days
find $BACKUP_DIR -name "*.db.gz" -mtime +30 -delete

echo "Backup completed: $DATE"
```

**Cron job for automated backups:**
```bash
# Add to crontab
0 2 * * * /usr/local/bin/currents-backup.sh
```

### Configuration Backup

```bash
#!/bin/bash
# /usr/local/bin/currents-config-backup.sh

BACKUP_DIR="/var/backups/currents/config"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p $BACKUP_DIR

# Backup configuration files
cp ~/.config/currents/config.toml $BACKUP_DIR/config_$DATE.toml
cp ~/.config/currents/orchestrator.toml $BACKUP_DIR/orchestrator_$DATE.toml

# Compress
tar -czf $BACKUP_DIR/currents_config_$DATE.tar.gz $BACKUP_DIR/*_$DATE.toml

# Cleanup
rm $BACKUP_DIR/*_$DATE.toml

echo "Configuration backup completed: $DATE"
```

### Recovery Procedures

**Database recovery:**
```bash
# Stop services
sudo systemctl stop currents@$USER
sudo systemctl stop currents-orchestrator@$USER

# Restore database
cp /var/backups/currents/weather_history_20240115_020000.db.gz ~/.config/currents/weather_history.db.gz
gunzip ~/.config/currents/weather_history.db.gz

# Restart services
sudo systemctl start currents@$USER
sudo systemctl start currents-orchestrator@$USER
```

**Configuration recovery:**
```bash
# Restore configuration
tar -xzf /var/backups/currents/currents_config_20240115_020000.tar.gz
cp config_20240115_020000.toml ~/.config/currents/config.toml
cp orchestrator_20240115_020000.toml ~/.config/currents/orchestrator.toml

# Restart services
sudo systemctl restart currents@$USER
sudo systemctl restart currents-orchestrator@$USER
```

---

## Scaling Considerations

### Horizontal Scaling

**Multiple orchestrator instances:**
```yaml
# docker-compose.scale.yml
version: '3.8'

services:
  currents-orchestrator-1:
    build: .
    environment:
      - ORCHESTRATOR_ID=1
      - LOCATIONS=london,tokyo
    volumes:
      - orchestrator-1-data:/var/lib/currents

  currents-orchestrator-2:
    build: .
    environment:
      - ORCHESTRATOR_ID=2
      - LOCATIONS=denver,new_york
    volumes:
      - orchestrator-2-data:/var/lib/currents

volumes:
  orchestrator-1-data:
  orchestrator-2-data:
```

### Database Scaling

**PostgreSQL for large deployments:**
```toml
# config.orchestrator.toml
[orchestrator]
storage_path = "postgresql://user:pass@localhost/currents"
database_pool_size = 10
database_timeout = 30
```

**Database sharding:**
```toml
[orchestrator.locations.london]
database_shard = "europe"

[orchestrator.locations.tokyo]
database_shard = "asia"

[orchestrator.locations.denver]
database_shard = "americas"
```

### Load Balancing

**API load balancing:**
```toml
[weather]
api_endpoints = [
    "https://api.openweathermap.org/data/2.5/weather",
    "https://api.openweathermap.org/data/2.5/weather"
]
load_balancing = "round_robin"
```

---

## Troubleshooting

### Common Production Issues

**1. High Memory Usage**
```bash
# Check memory usage
ps aux | grep currents
systemctl status currents@$USER

# Adjust memory limits
# Edit systemd service file:
MemoryLimit=512M
```

**2. Database Lock Issues**
```bash
# Check database locks
lsof ~/.config/currents/weather_history.db

# Restart service to clear locks
sudo systemctl restart currents@$USER
```

**3. API Rate Limiting**
```bash
# Check API usage
currents --api-stats

# Adjust limits in config
[weather]
api_daily_limit = 500  # Reduce limit
```

**4. Collector Process Issues**
```bash
# Check orchestrator health
currents-orchestrator --health

# Restart failed collectors
currents-orchestrator --restart-failed

# Check process status
currents-orchestrator --processes
```

### Performance Monitoring

**Resource monitoring:**
```bash
# Monitor CPU and memory
htop
iotop

# Monitor network usage
iftop

# Monitor disk usage
df -h
du -sh ~/.config/currents/
```

**Database performance:**
```bash
# Check database size
ls -lh ~/.config/currents/*.db

# Analyze database
currents-history stats

# Clean old data
currents-history clean 180  # Keep only 6 months
```

### Log Analysis

**Structured log analysis:**
```bash
# Filter error logs
journalctl -u currents@$USER | grep ERROR

# Monitor API calls
journalctl -u currents@$USER | grep "API call"

# Monitor alerts
journalctl -u currents@$USER | grep "Alert triggered"
```

**Log rotation:**
```bash
# Configure logrotate
cat > /etc/logrotate.d/currents << EOF
/var/log/currents/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 currents currents
}
EOF
```

---

## Production Checklist

### Pre-Deployment

- [ ] API keys configured and tested
- [ ] Database paths configured
- [ ] Log directories created
- [ ] Backup procedures tested
- [ ] Health checks implemented
- [ ] Monitoring configured
- [ ] Security settings applied

### Post-Deployment

- [ ] Services running and healthy
- [ ] API connectivity verified
- [ ] Database operations working
- [ ] Alerts functioning
- [ ] Logs being written
- [ ] Backups scheduled
- [ ] Monitoring alerts configured

### Maintenance

- [ ] Regular backup verification
- [ ] Log rotation working
- [ ] Database cleanup scheduled
- [ ] API usage monitoring
- [ ] Performance metrics tracking
- [ ] Security updates applied

---

**Currents** - Production-ready weather monitoring with enterprise-grade deployment options.
