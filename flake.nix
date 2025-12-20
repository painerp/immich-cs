{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rust-toolchain = pkgs.rust-bin.stable.latest.default.override { extensions = [ "rust-src" ]; };

        im-deploy = pkgs.rustPlatform.buildRustPackage {
          pname = "im-deploy";
          version = "0.1.0";

          src = ./im-deploy;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            rust-toolchain
          ];

          buildInputs = with pkgs; [
            openssl
            zlib
          ];

          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          meta = with pkgs.lib; {
            description = "TUI tool to help manage a immich deployment";
            license = licenses.mit;
            maintainers = [ painerp ];
          };
        };
      in
      {

        packages = {
          default = im-deploy;
          im-deploy = im-deploy;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rust-toolchain
            pkg-config
            autoconf
            openssl
            libtool
            automake
            clippy
          ];

          RUST_SRC_PATH = "${pkgs.rust-bin.stable.latest.default}/lib/rustlib/src/rust";
        };
      }
    );
}
