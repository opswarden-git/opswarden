{
  description = "OpsWarden -- dev environment (Rust, Node, Postgres+pgvector, quality tooling)";

  inputs = {
    nixpkgs.url     = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
        };

        # Postgres 16 with pgvector pre-loaded (for the RAG / AI SRE agent).
        pgWithVector = pkgs.postgresql_16.withPackages (p: [ p.pgvector ]);
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain

            # Web client
            nodejs

            # Database
            pgWithVector
            sqlx-cli

            # Build deps
            pkg-config
            openssl

            # Quality, coverage & profiling tooling
            cargo-watch
            cargo-nextest
            cargo-tarpaulin
            cargo-deny
            cargo-audit
            cargo-udeps
            cargo-bloat
            cargo-flamegraph
            cargo-modules
            cargo-depgraph
            graphviz
            tokei
            eza
            just
          ];

          shellHook = ''
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
            export RUST_LOG="''${RUST_LOG:-info}"
            if [[ -t 1 ]]; then
              echo ""
              echo "  OpsWarden dev shell ready -- run 'just' to list tasks"
              echo ""
            fi
          '';
        };
      });
}
