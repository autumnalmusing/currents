# Example Currents configuration for home-manager
# Add this to your home.nix or similar file

{
  services.currents = {
    enable = true;
    
    weather = {
      # Option 1: Use apiKeyFile (recommended for security)
      apiKeyFile = "/run/secrets/openweathermap-api-key";
      # Works with sops-nix, agenix, or just a plain file with restricted permissions
      
      # Option 2: Hardcode the key (not recommended for production)
      # apiKey = "your-api-key-here";
      
      # Note: apiKey and apiKeyFile are mutually exclusive
      
      location = "Denver,US";
      units = "metric";  # or "imperial" or "kelvin"
      provider = "openweathermap";  # or "weatherapi"
      apiDailyLimit = 1000;
    };
    
    notifications = {
      urgency = "normal";  # "low", "normal", or "critical"
      timeout = 5000;      # milliseconds
      sound = true;
    };
    
    polling = {
      intervalSeconds = 300;     # Check every 5 minutes
      retryAttempts = 3;
      retryDelaySeconds = 60;
    };
    
    # Optional: Customize forecast colors
    forecastHighlights = {
      temperature = {
        high = 30.0;          # Celsius (or Fahrenheit if using imperial)
        highColor = "#f38ba8"; # Hex color or named color like "red"
        low = 0.0;
        lowColor = "#89b4fa";  # Catppuccin Mocha blue
      };
      
      # Optional: Highlight humidity
      humidity = {
        high = 80.0;           # percentage
        highColor = "#89dceb"; # Catppuccin Mocha sky
        low = null;            # Don't highlight low humidity
        lowColor = "yellow";
      };
      
      # Optional: Highlight wind speed
      # windSpeed = {
      #   high = 15.0;         # m/s (or mph if imperial)
      #   highColor = "red";
      # };
      
      # Optional: Highlight precipitation
      # precipitation = {
      #   high = 70.0;         # percentage
      #   highColor = "#89b4fa";
      # };
    };
    
    # Alert rules
    alerts = [
      {
        name = "hot-weather";
        enabled = true;
        message = "It's getting hot! Temperature is above 30°C";
        repeat = "once";  # Only notify once per condition change
        condition = {
          temperature = {
            min = 30.0;
          };
        };
      }
      {
        name = "cold-weather";
        enabled = true;
        message = "Brrr! It's getting cold, temperature below 5°C";
        repeat = "3600";  # Notify every hour while condition persists
        condition = {
          temperature = {
            max = 5.0;
          };
        };
      }
      {
        name = "rain-alert";
        enabled = true;
        message = "Rain detected - don't forget your umbrella!";
        repeat = "once";
        condition = {
          description = "rain,drizzle,shower";
        };
      }
    ];
  };
}

