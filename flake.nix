{
  description = "shaderjoy";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [
          (import rust-overlay)
          (self: super: {
            rust-toolchain = self.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          })
        ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      with pkgs;
      {
        formatter = nixpkgs-fmt;

        devShell = mkShell rec {
          buildInputs = [
            pkg-config
            rust-toolchain
            rust-analyzer
            bacon
            cargo-edit

            udev 
            libxkbcommon
            vulkan-loader
            vulkan-validation-layers

            wayland


            xorg.libxcb
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath(buildInputs);
        };
      }
    );
}
