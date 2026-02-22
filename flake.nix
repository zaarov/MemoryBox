{
  description = "Rust dev shell";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-25.11";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      rustToolchain = pkgs.rust-bin.stable.latest.default;

      rustPlatform = pkgs.rustPlatform;
      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

    in {
      devShells.${system}.default = pkgs.mkShell {
        name = "rust-dev-shell";

        packages = [
          rustToolchain
          pkgs.rust-analyzer
          pkgs.pkg-config
          pkgs.openssl
          pkgs.openssl.dev
        ];

        shellHook = ''echo "🦀 Rust environment activated."'';
      };

      packages.${system} = {
        default = rustPlatform.buildRustPackage {
          pname = cargoToml.package.name;
          version = cargoToml.package.version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          buildInputs = with pkgs; [ openssl ];
          nativeBuildInputs = with pkgs; [ pkg-config openssl.dev ];
        };
      };
    };
}
