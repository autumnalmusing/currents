# Currents Home Manager Module Options

This document lists all available configuration options for the currents home-manager module.

## services.currents.enable

**Type:** `boolean`  
**Default:** `false`

Whether to enable the Currents weather alert daemon.

## services.currents.package

**Type:** `package`  
**Default:** `self.packages.${system}.default`

The currents package to use.

## Weather Configuration

### services.currents.weather.apiKey

**Type:** `null or string`  
**Default:** `null`

API key for weather provider (OpenWeatherMap or WeatherAPI).

> **Security Note:** Use `apiKeyFile` instead for better security. `apiKey` and `apiKeyFile` are mutually exclusive.

### services.currents.weather.apiKeyFile

**Type:** `null or path`  
**Default:** `null`  
**Example:** `"/run/secrets/openweathermap-api-key"`

Path to file containing the API key. **Recommended over `apiKey`** for security.

The file should contain only the API key with an optional trailing newline. Works great with:
- `sops-nix` - for encrypted secrets in git
- `agenix` - for age-encrypted secrets
- Plain files with restricted permissions (`chmod 600`)

**Example with sops-nix:**
```nix
sops.secrets.openweathermap-api-key = {
  sopsFile = ./secrets.yaml;
};

services.currents.weather.apiKeyFile = config.sops.secrets.openweathermap-api-key.path;
```

**Example with plain file:**
```bash
echo "your-api-key-here" > ~/.secrets/openweathermap-api-key
chmod 600 ~/.secrets/openweathermap-api-key
```
```nix
services.currents.weather.apiKeyFile = "${config.home.homeDirectory}/.secrets/openweathermap-api-key";
```

> **Note:** Either `apiKey` or `apiKeyFile` must be set, but not both.

### services.currents.weather.location

**Type:** `string`  
**Example:** `"London,UK"` or `"Denver,US"`

Location for weather data.

### services.currents.weather.units

**Type:** `enum [ "metric" "imperial" "kelvin" ]`  
**Default:** `"metric"`

Unit system for temperature and other measurements.

### services.currents.weather.provider

**Type:** `enum [ "openweathermap" "weatherapi" ]`  
**Default:** `"openweathermap"`

Weather API provider to use.

### services.currents.weather.apiDailyLimit

**Type:** `integer`  
**Default:** `1000`

Maximum API calls allowed per day. Prevents unexpected charges.

## Notification Configuration

### services.currents.notifications.urgency

**Type:** `enum [ "low" "normal" "critical" ]`  
**Default:** `"normal"`

Notification urgency level for desktop notifications.

### services.currents.notifications.timeout

**Type:** `integer`  
**Default:** `5000`

Notification timeout in milliseconds.

### services.currents.notifications.sound

**Type:** `boolean`  
**Default:** `true`

Whether to play sound with notifications.

## Polling Configuration

### services.currents.polling.intervalSeconds

**Type:** `integer`  
**Default:** `300`

Polling interval in seconds (how often to check weather).

### services.currents.polling.retryAttempts

**Type:** `integer`  
**Default:** `3`

Number of retry attempts on API failure.

### services.currents.polling.retryDelaySeconds

**Type:** `integer`  
**Default:** `60`

Delay between retry attempts in seconds.

## Cache Configuration

### services.currents.cache.path

**Type:** `string`  
**Default:** `"${config.home.homeDirectory}/.cache/currents/weather.json"`

Path to cached weather snapshot (used by Waybar integration).

### services.currents.cache.ttlSeconds

**Type:** `integer`  
**Default:** `300`

Time-to-live for cached data in seconds.

## Forecast Highlighting

### Temperature Highlighting

#### services.currents.forecastHighlights.temperature.high

**Type:** `null or float`  
**Default:** `30.0`

Temperature threshold for high temperature highlighting (in configured units).

#### services.currents.forecastHighlights.temperature.highColor

**Type:** `string`  
**Default:** `"red"`  
**Example:** `"#f38ba8"` or `"red"`

Color for high temperatures. Can be named color or hex code.

#### services.currents.forecastHighlights.temperature.low

**Type:** `null or float`  
**Default:** `0.0`

Temperature threshold for low temperature highlighting.

#### services.currents.forecastHighlights.temperature.lowColor

**Type:** `string`  
**Default:** `"blue"`  
**Example:** `"#89b4fa"`

Color for low temperatures.

### Humidity Highlighting

Similar structure to temperature, with options:
- `humidity.high` (default: `null`)
- `humidity.highColor` (default: `"red"`)
- `humidity.low` (default: `null`)
- `humidity.lowColor` (default: `"yellow"`)

### Wind Speed Highlighting

Similar structure, with options:
- `windSpeed.high` (default: `null`)
- `windSpeed.highColor` (default: `"red"`)
- `windSpeed.low` (default: `null`)
- `windSpeed.lowColor` (default: `"cyan"`)

### Precipitation Highlighting

Similar structure, with options:
- `precipitation.high` (default: `null`)
- `precipitation.highColor` (default: `"magenta"`)
- `precipitation.low` (default: `null`)
- `precipitation.lowColor` (default: `"green"`)

## Alert Rules

### services.currents.alerts

**Type:** `list of submodule`  
**Default:** `[]`

List of alert rules to monitor weather conditions.

Each alert has the following options:

#### name

**Type:** `string`

Unique name for the alert rule.

#### enabled

**Type:** `boolean`  
**Default:** `true`

Whether this alert is enabled.

#### message

**Type:** `string`

Alert message to display in the notification.

#### repeat

**Type:** `string`  
**Default:** `"once"`  
**Examples:** `"once"`, `"always"`, `"3600"`

How often to repeat the alert:
- `"once"`: Only notify on first trigger, not again until conditions return to normal
- `"always"`: Notify every polling interval while condition persists
- `"<seconds>"`: Notify every N seconds while condition persists (e.g., `"3600"` for hourly)

#### condition

**Type:** `attribute set`

Alert condition specification. Can include:

**Temperature conditions:**
```nix
condition = {
  temperature = {
    min = 30.0;  # Alert when temp >= 30
    max = 5.0;   # Alert when temp <= 5
  };
};
```

**Humidity conditions:**
```nix
condition = {
  humidity = {
    min = 80.0;
    max = 20.0;
  };
};
```

**Wind speed conditions:**
```nix
condition = {
  wind_speed = {
    min = 15.0;
  };
};
```

**Precipitation conditions:**
```nix
condition = {
  precipitation = {
    intensity = "heavy";  # "light", "moderate", "heavy"
    probability = 0.7;    # 0.0 to 1.0
  };
};
```

**Description-based conditions:**
```nix
condition = {
  description = "rain,drizzle,shower";  # Comma-separated keywords
};
```

Multiple conditions can be combined - all must be true for the alert to trigger.

## Available Colors

Colors can be specified as:
- **Named colors:** `"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"`, `"cyan"`, `"white"`
  - These respect your terminal's color scheme
- **Hex colors:** `"#ff5733"`, `"#d3c6aa"`, `"#f00"` (3 or 6 digit hex codes)
  - These display exact RGB values if your terminal supports it

## Complete Example

See `nix/example-config.nix` for a complete working example with all options demonstrated.

