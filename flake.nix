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
      
      # Base package configuration
      mkCurrentsPackage = { pname, cargoBuildFlags ? [], cargoTestFlags ? [] }: 
        pkgs.rustPlatform.buildRustPackage {
          inherit pname;
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          
          inherit cargoBuildFlags cargoTestFlags;
          
          # Build inputs
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl ];
          
          # Environment variables for build
          OPENSSL_NO_VENDOR = 1;
        };
    in
    {
      packages.${system} = {
        # Just the daemon (minimal install)
        currents-daemon = mkCurrentsPackage {
          pname = "currents-daemon";
          cargoBuildFlags = [ "-p" "currents-daemon" ];
          cargoTestFlags = [ "-p" "currents-daemon" "-p" "currents-core" ];
        };
        
        # Just the forecast tool
        currents-forecast = mkCurrentsPackage {
          pname = "currents-forecast";
          cargoBuildFlags = [ "-p" "currents-forecast" ];
          cargoTestFlags = [ "-p" "currents-forecast" "-p" "currents-core" ];
        };
        
        # Both binaries (default)
        currents-full = mkCurrentsPackage {
          pname = "currents-full";
          cargoBuildFlags = [ "--workspace" ];
          cargoTestFlags = [ "--workspace" ];
        };
        
        # Default is just the daemon (most users only need this)
        default = self.packages.${system}.currents-daemon;
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
      homeManagerModules.currents = import ./nix/home-manager-module.nix {
        inherit self system;
      };
    };
}
