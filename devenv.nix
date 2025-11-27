{pkgs, ...}: {
  # Name of the project with version
  name = "streamweave";

  # Languages
  languages = {
    rust = {
      enable = true;
      channel = "stable";
      components = [
        "cargo"
        "clippy"
        "rust-analyzer"
        "rustc"
        "rustfmt"
        "llvm-tools"
      ];
      targets = [
        "wasm32-unknown-unknown"
      ];
    };
  };

  # Development packages
  packages = with pkgs; [
    # Rust tools
    clippy
    rust-analyzer
    rustc
    rustfmt
    wasm-bindgen-cli
    wasm-pack

    # Development tools
    direnv
    pre-commit

    # Formatting tools
    alejandra

    # Publishing tools
    cargo-watch
    cargo-audit
    cargo-llvm-cov
    cargo-nextest

    # Documentation tools
    mdbook
    mdbook-mermaid
    # Doxidize will be installed via cargo install in devenv

    # Version management
    git
    gitAndTools.gh

    # Build dependencies for Kafka integration
    cmake
    pkg-config
    openssl
    zlib
    zstd
  ];

  # Pre-commit hooks
  git-hooks = {
    hooks = {
      cargo-check = {
        enable = true;
      };

      clippy = {
        enable = true;
      };

      rustfmt = {
        enable = true;
      };

      # Use our unified pre-commit script
      our-pre-commit-hook = {
        enable = true;
        name = "pre-commit";
        description = "Run all pre-commit checks";
        entry = "bin/pre-commit";
        language = "system";
        pass_filenames = false;
      };
    };
  };

  # Services
  services.kafka = {
    enable = true;
  };

  # Environment variables
  env = {
    RUST_BACKTRACE = "1";
    CARGO_TERM_COLOR = "always";
  };
}
