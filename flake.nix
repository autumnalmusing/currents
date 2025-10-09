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
      homeManagerModules.currents = import ./nix/home-manager-module.nix {
        inherit self system;
      };
    };
}
