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

        # WebKit/GTK stack the Tauri v2 desktop shell links against on NixOS.
        # Ported from the proven hello-world baseline (docs/code/05-...).
        tauriLibraries = with pkgs; [
          webkitgtk_4_1 gtk3 cairo gdk-pixbuf glib dbus openssl librsvg
          libsoup_3 at-spi2-atk libdbusmenu-gtk3 glib-networking
        ];
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

        # Desktop (Tauri v2) shell: the default shell plus the WebKit/GTK stack
        # and GTK runtime env. Enter with `nix develop .#tauri`. The pkg-config
        # paths for webkit2gtk-4.1 et al. come from buildInputs automatically;
        # we do NOT pin PKG_CONFIG_PATH here (that would hide them).
        devShells.tauri = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            nodejs

            # Tauri/WebKit/GTK runtime + dev libraries
            dbus
            openssl
            librsvg
            libsoup_3
            webkitgtk_4_1
            at-spi2-atk
            gtk3
            libdbusmenu-gtk3
            glib-networking
            gsettings-desktop-schemas
            adwaita-icon-theme
            dconf
            cairo
            gdk-pixbuf
            glib
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            gobject-introspection
          ];

          shellHook = ''
            export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath tauriLibraries}:$LD_LIBRARY_PATH
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-desktop-schemas:${pkgs.gtk3}/share/gsettings-desktop-schemas:${pkgs.gtk3}/share:${pkgs.gsettings-desktop-schemas}/share:${pkgs.adwaita-icon-theme}/share:$XDG_DATA_DIRS"
            export GSETTINGS_SCHEMA_DIR="${pkgs.gtk3}/share/glib-2.0/schemas:${pkgs.gsettings-desktop-schemas}/share/glib-2.0/schemas:$GSETTINGS_SCHEMA_DIR"
            export GIO_EXTRA_MODULES="${pkgs.dconf.lib}/lib/gio/modules:${pkgs.glib-networking}/lib/gio/modules"
            export GTK_CSD=1
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            if [[ -t 1 ]]; then
              echo ""
              echo "  OpsWarden Tauri shell ready -- cd client-desktop && npm run tauri dev"
              echo ""
            fi
          '';
        };
      });
}
