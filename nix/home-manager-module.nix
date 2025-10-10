{ self, system }:

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.currents;
  
  tomlFormat = pkgs.formats.toml { };
  
  # Get API key from file if apiKeyFile is set, otherwise use apiKey
  apiKey = 
    if cfg.weather.apiKeyFile != null 
    then lib.removeSuffix "\n" (builtins.readFile cfg.weather.apiKeyFile)
    else cfg.weather.apiKey;
  
  # Helper to build highlight config without null values
  buildHighlight = highlight: 
    filterAttrs (n: v: v != null) {
      high = highlight.high;
      high_color = highlight.highColor;
      low = highlight.low;
      low_color = highlight.lowColor;
    };
  
  configFile = tomlFormat.generate "config.toml" {
    weather = {
      api_key = apiKey;
      location = cfg.weather.location;
      units = cfg.weather.units;
      provider = cfg.weather.provider;
      api_daily_limit = cfg.weather.apiDailyLimit;
    };
    
    notifications = {
      urgency = cfg.notifications.urgency;
      timeout = cfg.notifications.timeout;
      sound = cfg.notifications.sound;
    };
    
    polling = {
      interval_seconds = cfg.polling.intervalSeconds;
      retry_attempts = cfg.polling.retryAttempts;
      retry_delay_seconds = cfg.polling.retryDelaySeconds;
    };
    
    cache = {
      path = cfg.cache.path;
      ttl_seconds = cfg.cache.ttlSeconds;
    };
    
    forecast_highlights = {
      temperature = buildHighlight cfg.forecastHighlights.temperature;
    } // optionalAttrs (cfg.forecastHighlights.humidity.high != null || cfg.forecastHighlights.humidity.low != null) {
      humidity = buildHighlight cfg.forecastHighlights.humidity;
    } // optionalAttrs (cfg.forecastHighlights.windSpeed.high != null || cfg.forecastHighlights.windSpeed.low != null) {
      wind_speed = buildHighlight cfg.forecastHighlights.windSpeed;
    } // optionalAttrs (cfg.forecastHighlights.precipitation.high != null || cfg.forecastHighlights.precipitation.low != null) {
      precipitation = buildHighlight cfg.forecastHighlights.precipitation;
    } // optionalAttrs (cfg.forecastHighlights.pressure.high != null || cfg.forecastHighlights.pressure.low != null) {
      pressure = buildHighlight cfg.forecastHighlights.pressure;
    } // optionalAttrs (cfg.forecastHighlights.visibility.high != null || cfg.forecastHighlights.visibility.low != null) {
      visibility = buildHighlight cfg.forecastHighlights.visibility;
    } // optionalAttrs (cfg.forecastHighlights.uvIndex.high != null || cfg.forecastHighlights.uvIndex.low != null) {
      uv_index = buildHighlight cfg.forecastHighlights.uvIndex;
    } // optionalAttrs (cfg.forecastHighlights.cloudCover.high != null || cfg.forecastHighlights.cloudCover.low != null) {
      cloud_cover = buildHighlight cfg.forecastHighlights.cloudCover;
    } // optionalAttrs (cfg.forecastHighlights.aqi.high != null || cfg.forecastHighlights.aqi.low != null) {
      aqi = buildHighlight cfg.forecastHighlights.aqi;
    };
    
    forecast_display = {
      show_date = cfg.forecastDisplay.showDate;
      show_weather = cfg.forecastDisplay.showWeather;
      show_temp = cfg.forecastDisplay.showTemp;
      show_humidity = cfg.forecastDisplay.showHumidity;
      show_wind = cfg.forecastDisplay.showWind;
      show_precip = cfg.forecastDisplay.showPrecip;
      show_pressure = cfg.forecastDisplay.showPressure;
      show_visibility = cfg.forecastDisplay.showVisibility;
      show_uv = cfg.forecastDisplay.showUv;
      show_clouds = cfg.forecastDisplay.showClouds;
      show_wind_dir = cfg.forecastDisplay.showWindDir;
      show_aqi = cfg.forecastDisplay.showAqi;
    };
    
    alerts = map (alert: {
      name = alert.name;
      enabled = alert.enabled;
      message = alert.message;
      repeat = alert.repeat;
      condition = alert.condition;
    }) cfg.alerts;
  };

in {
  options.services.currents = {
    enable = mkEnableOption "Currents weather alert daemon";
    
    package = mkOption {
      type = types.package;
      default = self.packages.${system}.currents-daemon;
      description = ''
        The currents package to use.
        Options:
        - currents-daemon (default) - Just the daemon for alerts
        - currents-forecast - Just the forecast display tool
        - currents-full - Both daemon and forecast
      '';
      example = literalExpression "inputs.currents.packages.\${system}.currents-full";
    };
    
    weather = {
      apiKey = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "API key for weather provider. Use apiKeyFile instead for better security.";
      };
      
      apiKeyFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        example = "/run/secrets/currents-api-key";
        description = "Path to file containing the API key. Preferred over apiKey for security.";
      };
      
      location = mkOption {
        type = types.str;
        example = "London,UK";
        description = "Location for weather data";
      };
      
      units = mkOption {
        type = types.enum [ "metric" "imperial" "kelvin" ];
        default = "metric";
        description = "Unit system for temperature";
      };
      
      provider = mkOption {
        type = types.enum [ "openweathermap" "weatherapi" ];
        default = "openweathermap";
        description = "Weather API provider";
      };
      
      apiDailyLimit = mkOption {
        type = types.int;
        default = 1000;
        description = "Maximum API calls allowed per day";
      };
    };
    
    notifications = {
      urgency = mkOption {
        type = types.enum [ "low" "normal" "critical" ];
        default = "normal";
        description = "Notification urgency level";
      };
      
      timeout = mkOption {
        type = types.int;
        default = 5000;
        description = "Notification timeout in milliseconds";
      };
      
      sound = mkOption {
        type = types.bool;
        default = true;
        description = "Play sound with notifications";
      };
    };
    
    polling = {
      intervalSeconds = mkOption {
        type = types.int;
        default = 300;
        description = "Polling interval in seconds";
      };
      
      retryAttempts = mkOption {
        type = types.int;
        default = 3;
        description = "Number of retry attempts on failure";
      };
      
      retryDelaySeconds = mkOption {
        type = types.int;
        default = 60;
        description = "Delay between retry attempts in seconds";
      };
    };
    
    cache = {
      path = mkOption {
        type = types.str;
        default = "${config.home.homeDirectory}/.cache/currents/weather.json";
        description = "Path to cached weather snapshot";
      };
      
      ttlSeconds = mkOption {
        type = types.int;
        default = 300;
        description = "Time-to-live for cached data in seconds";
      };
    };
    
    forecastHighlights = {
      temperature = {
        high = mkOption {
          type = types.nullOr types.float;
          default = 30.0;
          description = "Temperature threshold for high temperature highlighting";
        };
        
        highColor = mkOption {
          type = types.str;
          default = "red";
          example = "#ff5733";
          description = "Color for high temperatures (named color or hex code)";
        };
        
        low = mkOption {
          type = types.nullOr types.float;
          default = 0.0;
          description = "Temperature threshold for low temperature highlighting";
        };
        
        lowColor = mkOption {
          type = types.str;
          default = "blue";
          example = "#89b4fa";
          description = "Color for low temperatures (named color or hex code)";
        };
      };
      
      humidity = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Humidity threshold for highlighting";
        };
        
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high humidity";
        };
        
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low humidity threshold for highlighting";
        };
        
        lowColor = mkOption {
          type = types.str;
          default = "yellow";
          description = "Color for low humidity";
        };
      };
      
      windSpeed = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Wind speed threshold for highlighting";
        };
        
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high wind speed";
        };
        
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low wind speed threshold for highlighting";
        };
        
        lowColor = mkOption {
          type = types.str;
          default = "cyan";
          description = "Color for low wind speed";
        };
      };
      
      precipitation = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Precipitation probability threshold for highlighting (percentage)";
        };
        
        highColor = mkOption {
          type = types.str;
          default = "magenta";
          description = "Color for high precipitation probability";
        };
        
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low precipitation probability threshold for highlighting";
        };
        
        lowColor = mkOption {
          type = types.str;
          default = "green";
          description = "Color for low precipitation probability";
        };
      };
      
      # New fields added in workspace refactoring
      pressure = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Pressure threshold for highlighting (hPa/mb)";
        };
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high pressure";
        };
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low pressure threshold";
        };
        lowColor = mkOption {
          type = types.str;
          default = "yellow";
          description = "Color for low pressure";
        };
      };
      
      visibility = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Visibility threshold for highlighting (km)";
        };
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high visibility";
        };
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low visibility threshold";
        };
        lowColor = mkOption {
          type = types.str;
          default = "yellow";
          description = "Color for low visibility";
        };
      };
      
      uvIndex = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "UV index threshold for highlighting";
        };
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high UV";
        };
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low UV threshold";
        };
        lowColor = mkOption {
          type = types.str;
          default = "yellow";
          description = "Color for low UV";
        };
      };
      
      cloudCover = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Cloud cover threshold for highlighting (percentage)";
        };
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high cloud cover";
        };
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low cloud cover threshold";
        };
        lowColor = mkOption {
          type = types.str;
          default = "yellow";
          description = "Color for low cloud cover";
        };
      };
      
      aqi = {
        high = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Air Quality Index threshold (US EPA 1-6 scale)";
        };
        highColor = mkOption {
          type = types.str;
          default = "red";
          description = "Color for high AQI (poor air quality)";
        };
        low = mkOption {
          type = types.nullOr types.float;
          default = null;
          description = "Low AQI threshold (good air quality)";
        };
        lowColor = mkOption {
          type = types.str;
          default = "green";
          description = "Color for low AQI";
        };
      };
    };
    
    forecastDisplay = {
      showDate = mkOption {
        type = types.bool;
        default = true;
        description = "Show date column in forecast";
      };
      showWeather = mkOption {
        type = types.bool;
        default = true;
        description = "Show weather description column";
      };
      showTemp = mkOption {
        type = types.bool;
        default = true;
        description = "Show temperature range column";
      };
      showHumidity = mkOption {
        type = types.bool;
        default = true;
        description = "Show humidity column";
      };
      showWind = mkOption {
        type = types.bool;
        default = true;
        description = "Show wind speed column";
      };
      showPrecip = mkOption {
        type = types.bool;
        default = true;
        description = "Show precipitation probability column";
      };
      showPressure = mkOption {
        type = types.bool;
        default = false;
        description = "Show atmospheric pressure column";
      };
      showVisibility = mkOption {
        type = types.bool;
        default = false;
        description = "Show visibility column";
      };
      showUv = mkOption {
        type = types.bool;
        default = false;
        description = "Show UV index column";
      };
      showClouds = mkOption {
        type = types.bool;
        default = false;
        description = "Show cloud cover column";
      };
      showWindDir = mkOption {
        type = types.bool;
        default = false;
        description = "Show wind direction column";
      };
      showAqi = mkOption {
        type = types.bool;
        default = false;
        description = "Show air quality index column";
      };
    };
    
    alerts = mkOption {
      type = types.listOf (types.submodule {
        options = {
          name = mkOption {
            type = types.str;
            description = "Alert rule name";
          };
          
          enabled = mkOption {
            type = types.bool;
            default = true;
            description = "Whether this alert is enabled";
          };
          
          message = mkOption {
            type = types.str;
            description = "Alert message to display";
          };
          
          repeat = mkOption {
            type = types.str;
            default = "once";
            example = "3600";
            description = "How often to repeat: 'once', 'always', or duration in seconds";
          };
          
          condition = mkOption {
            type = types.attrs;
            description = "Alert condition (temperature, humidity, description, etc.)";
          };
        };
      });
      default = [];
      description = "List of alert rules";
    };
  };
  
  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.weather.apiKey != null || cfg.weather.apiKeyFile != null;
        message = "services.currents.weather: either apiKey or apiKeyFile must be set";
      }
      {
        assertion = !(cfg.weather.apiKey != null && cfg.weather.apiKeyFile != null);
        message = "services.currents.weather: apiKey and apiKeyFile are mutually exclusive";
      }
    ];
    
    home.packages = [ cfg.package ];
    
    xdg.configFile."currents/config.toml".source = configFile;
    
    systemd.user.services.currents = {
      Unit = {
        Description = "Currents weather alert daemon";
        After = [ "graphical-session.target" ];
      };
      
      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/currents --config ${config.xdg.configHome}/currents/config.toml";
        Restart = "always";
        RestartSec = 10;
        Environment = "RUST_LOG=info";
        
        # Security settings
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = "read-only";
        ReadWritePaths = [
          "${config.xdg.configHome}/currents"
          "${config.xdg.cacheHome}/currents"
        ];
        
        # Resource limits
        MemoryMax = "64M";
        CPUQuota = "10%";
      };
      
      Install.WantedBy = [ "default.target" ];
    };
  };
}

