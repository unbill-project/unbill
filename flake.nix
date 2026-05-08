{
  description = "Unbill — shared bill tracking";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      crane,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;

        commonArgs = {
          pname = "unbill";
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;
          # Only resolve deps for the two packages we care about, avoiding
          # gtk/glib/webkit2gtk from unbill-tauri.
          cargoExtraArgs = "-p unbill-cli -p unbill-tui";
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
            pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          ];
        };

        # Build dependencies for the workspace minus tauri, shared between packages.
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        unbill-cli = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--bin unbill-cli";
            doCheck = false;
          }
        );

        unbill-tui = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            cargoExtraArgs = "--bin unbill-tui";
            doCheck = false;
          }
        );
      in
      {
        packages = {
          inherit unbill-cli unbill-tui;
          default = unbill-cli;
        };
      }
    );
}
