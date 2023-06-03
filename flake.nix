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
        libPath = with pkgs; lib.makeLibraryPath [
          libxkbcommon

          wayland

          vulkan-loader
          vulkan-validation-layers
        ];

      in
      with pkgs;
      {
        formatter = nixpkgs-fmt;

        devShell = mkShell {
          buildInputs = [
            pkgconfig
            rust-toolchain
            rust-analyzer
            bacon
            cargo-edit

            xorg.libxcb
          ];

          LD_LIBRARY_PATH = libPath;
          VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
        };
      }
    );
}
