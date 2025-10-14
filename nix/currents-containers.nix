# Currents OCI Containers Configuration
# For NixOS using virtualisation.oci-containers.containers

{ config, lib, pkgs, ... }:

let
  # Configuration
  weatherapiKey = "your-weatherapi-key-here";
  
  # Generate orchestrator configuration
  orchestratorConfig = pkgs.writeText "orchestrator.toml" ''
    [orchestrator]
    storage_path = "/var/lib/currents/orchestrator.db"
    analysis_interval = 3600
    health_check_interval = 300

    [orchestrator.locations.london]
    name = "London, UK"
    coordinates = [51.5074, -0.1278]
    weather.api_key = "${weatherapiKey}"
    weather.provider = "weatherapi"
    weather.units = "metric"
    weather.collection_interval = 1800
    collection_strategy = "interval"

    [orchestrator.locations.tokyo]
    name = "Tokyo, JP"
    coordinates = [35.6762, 139.6503]
    weather.api_key = "${weatherapiKey}"
    weather.provider = "weatherapi"
    weather.units = "metric"
    weather.collection_interval = 1800
    collection_strategy = "interval"
  '';

  # Management script
  managementScript = pkgs.writeShellScript "manage-currents.sh" ''
    #!/bin/bash
    set -euo pipefail

    case "''${1:-help}" in
      start)
        echo "🚀 Starting Currents weather monitoring..."
        sudo systemctl start container@currents-orchestrator
        echo "✅ Services started!"
        ;;
      stop)
        echo "🛑 Stopping Currents weather monitoring..."
        sudo systemctl stop container@currents-orchestrator
        echo "✅ Services stopped!"
        ;;
      restart)
        echo "🔄 Restarting Currents weather monitoring..."
        sudo systemctl restart container@currents-orchestrator
        echo "✅ Services restarted!"
        ;;
      logs)
        echo "📋 Showing logs..."
        sudo journalctl -u container@currents-orchestrator -f
        ;;
      forecast)
        echo "🌤️  Showing weather forecast..."
        sudo podman exec currents-orchestrator currents-forecast
        ;;
      history)
        echo "📊 Showing weather history..."
        sudo podman exec currents-orchestrator currents-history history 7d
        ;;
      status)
        echo "📊 Service status:"
        sudo systemctl status container@currents-orchestrator
        echo ""
        echo "💾 Database info:"
        sudo podman exec currents-orchestrator currents-history stats 2>/dev/null || echo "Database not ready yet"
        ;;
      backup)
        echo "💾 Backing up database..."
        BACKUP_DIR="/var/lib/currents/backups/$(date +%Y%m%d_%H%M%S)"
        sudo mkdir -p "$BACKUP_DIR"
        sudo podman exec currents-orchestrator cp /var/lib/currents/orchestrator.db /tmp/backup.db
        sudo podman cp currents-orchestrator:/tmp/backup.db "$BACKUP_DIR/orchestrator.db"
        echo "✅ Backup created in $BACKUP_DIR"
        ;;
      help|*)
        echo "🌤️  Currents Weather Monitoring Management"
        echo ""
        echo "Usage: $0 <command>"
        echo ""
        echo "Commands:"
        echo "  start      - Start weather monitoring services"
        echo "  stop       - Stop all services"
        echo "  restart    - Restart services"
        echo "  logs       - Show orchestrator logs"
        echo "  forecast   - Show weather forecast"
        echo "  history    - Show weather history"
        echo "  status     - Show service status and database info"
        echo "  backup     - Backup database"
        echo "  help       - Show this help"
        ;;
    esac
  '';

in
{
  # Enable OCI containers
  virtualisation.oci-containers.backend = "podman";
  
  # Define the orchestrator container
  virtualisation.oci-containers.containers.currents-orchestrator = {
    image = "currents-orchestrator:latest";
    imageFile = pkgs.dockerTools.buildImage {
      name = "currents-orchestrator";
      tag = "latest";
      contents = [
        pkgs.bash
        pkgs.coreutils
        pkgs.findutils
        pkgs.gnugrep
        pkgs.gnused
        pkgs.gawk
        pkgs.curl
        pkgs.jq
      ];
      config = {
        Cmd = [ "currents-orchestrator" "/etc/currents/orchestrator.toml" ];
        Env = [
          "RUST_LOG=info"
          "RUST_LOG_FORMAT=json"
        ];
        ExposedPorts = {
          "8080/tcp" = {};
        };
        WorkingDir = "/var/lib/currents";
        User = "currents";
      };
    };
    
    autoStart = true;
    extraOptions = [
      "--name=currents-orchestrator"
      "--restart=unless-stopped"
      "--user=currents:currents"
      "--workdir=/var/lib/currents"
      "--env=RUST_LOG=info"
      "--env=RUST_LOG_FORMAT=json"
      "--volume=/var/lib/currents:/var/lib/currents:Z"
      "--volume=/var/lib/currents/logs:/var/log/currents:Z"
      "--volume=${orchestratorConfig}:/etc/currents/orchestrator.toml:ro,Z"
      "--health-cmd=currents-orchestrator --health"
      "--health-interval=30s"
      "--health-timeout=10s"
      "--health-retries=3"
      "--health-start-period=10s"
    ];
  };

  # User and group for currents
  users.users.currents = {
    isSystemUser = true;
    group = "currents";
    home = "/var/lib/currents";
    createHome = true;
    shell = pkgs.bash;
  };
  
  users.groups.currents = {};

  # Create directories
  systemd.tmpfiles.rules = [
    "d /var/lib/currents 0755 currents currents -"
    "d /var/lib/currents/logs 0755 currents currents -"
    "d /var/lib/currents/backups 0755 currents currents -"
    "d /var/lib/currents/config 0755 currents currents -"
  ];

  # Copy configuration file
  systemd.services.currents-config = {
    description = "Copy Currents configuration";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${pkgs.coreutils}/bin/cp ${orchestratorConfig} /var/lib/currents/config/orchestrator.toml";
      ExecStartPost = "${pkgs.coreutils}/bin/chown currents:currents /var/lib/currents/config/orchestrator.toml";
      ExecStartPost = "${pkgs.coreutils}/bin/chmod 644 /var/lib/currents/config/orchestrator.toml";
    };
    wantedBy = [ "multi-user.target" ];
    before = [ "container@currents-orchestrator.service" ];
  };

  # Management script
  environment.systemPackages = with pkgs; [
    (pkgs.writeShellScriptBin "manage-currents" (builtins.readFile managementScript))
  ];

  # Environment variables
  environment.variables = {
    CURRENTS_CONFIG_PATH = "/var/lib/currents/config/orchestrator.toml";
    CURRENTS_DATA_PATH = "/var/lib/currents";
  };

  # Enable Podman
  virtualisation.podman.enable = true;
  virtualisation.podman.dockerCompat = true;
  virtualisation.podman.dockerSocket.enable = true;

  # Firewall rules (if needed)
  networking.firewall.allowedTCPPorts = [ 8080 ];
}
