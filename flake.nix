{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs:
    (inputs.flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.default
          ];
        };

        inherit (builtins) readFile fromTOML;

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;
        # rustfmt from rust-nightly used for advanced options in rustfmt
        rustfmt-nightly = pkgs.rust-bin.nightly.latest.rustfmt;

        shellPkgs = [
          rustfmt-nightly # must come before `rust` to so this version of rustfmt is first in PATH
          rust
        ] ++ (with pkgs; [
          cargo-sort
          just
          nixpkgs-fmt
          present-cli
        ]);

        bashguard = pkgs.rustPlatform.buildRustPackage {
          inherit ((fromTOML (readFile ./Cargo.toml)).package)
            name
            version
            ;
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type: !(pkgs.lib.hasSuffix ".nix" path);
          };
          cargoHash = "sha256-moeZzA3b97ZDH8x4ZFGK75CeeuzIQrBziR/5s+DKu+Q=";
        };

      in
      {
        devShells = {
          default = pkgs.mkShell {
            buildInputs = shellPkgs;
          };
        };

        packages = {
          default = bashguard;

          bashguard = bashguard;
        };
      }));
}
