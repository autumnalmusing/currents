{
  description = "Currents, a weather alert daemon";
  
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    home-manager.url = "github:nix-community/home-manager";
  };
  
  outputs = { self, nixpkgs, home-manager }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "currents";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        
        # Build inputs
        nativeBuildInputs = with pkgs; [ pkg-config ];
        buildInputs = with pkgs; [ openssl ];
        
        # Environment variables for build
        OPENSSL_NO_VENDOR = 1;
      };
      
      # NixOS module
      nixosModules.currents = { config, lib, pkgs, ... }:
        with lib;
        let cfg = config.services.currents;
        in {
          options.services.currents = {
            enable = mkEnableOption "Currents, a weather alert daemon";
          };
          config = mkIf cfg.enable {
            systemd.services."currents@" = {
              # ... existing service config
            };
          };
        };
      
      # Home Manager module
      homeManagerModules.currents = { config, lib, pkgs, ... }:
        with lib;
        let cfg = config.services.currents;
        in {
          options.services.currents = {
            enable = mkEnableOption "Currents, a weather alert daemon";
            package = mkOption {
              type = types.package;
              default = self.packages.${system}.default;
              description = "The currents package to use";
            };
          };
          config = mkIf cfg.enable {
            home.packages = [ cfg.package ];
            systemd.user.services.currents = {
              Unit = {
                Description = "Currents, a weather alert daemon";
                After = [ "graphical-session.target" ];
              };
              Service = {
                Type = "simple";
                ExecStart = "${cfg.package}/bin/currents";
                Restart = "always";
                RestartSec = 10;
                Environment = "RUST_LOG=info";
                
                # Security settings
                NoNewPrivileges = true;
                PrivateTmp = true;
                ProtectSystem = "strict";
                ProtectHome = "read-only";
                ReadWritePaths = [ "%h/.config/currents" "%h/.cache/currents" ];
                
                # Resource limits
                MemoryMax = "64M";
                CPUQuota = "10%";
              };
              Install.WantedBy = [ "default.target" ];
            };
          };
        };
    };
}
