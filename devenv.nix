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

      # Build check
      build = {
        enable = true;
        name = "cargo-build";
        description = "Check if project builds successfully";
        entry = "bin/build";
        pass_filenames = false;
      };

      build-wasm = {
        enable = true;
        name = "cargo-build-wasm";
        description = "Check if WASM project builds successfully";
        entry = "bin/build-wasm";
        pass_filenames = false;
      };

      # Test
      test = {
        enable = true;
        name = "cargo-test";
        description = "Run cargo tests";
        entry = "bin/test";
        pass_filenames = false;
      };

      # Security audit
      # Note: We ignore RUSTSEC-2023-0071 (rsa 0.9.9) and RUSTSEC-2024-0436 (paste 1.0.15)
      # See cargo-audit.toml for justification
      audit = {
        enable = true;
        name = "cargo-audit";
        description = "Run security audit";
        entry = "cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2024-0436";
        pass_filenames = false;
      };

      # Documentation check (optional - can be disabled if too strict)
      docs = {
        enable = false; # Set to true to enable documentation checks
        name = "cargo-doc-check";
        description = "Check that documentation builds and has no warnings";
        entry = "cargo doc --all-features --no-deps 2>&1 | grep -i 'warning.*missing' || true";
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
