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

    # Version management
    git
    gitAndTools.gh
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

      # Test
      test = {
        enable = true;
        name = "cargo-test";
        description = "Run cargo tests";
        entry = "bin/test";
        pass_filenames = false;
      };

      # Security audit
      audit = {
        enable = true;
        name = "cargo-audit";
        description = "Run security audit";
        entry = "cargo audit";
        pass_filenames = false;
      };
    };
  };

  # Environment variables
  env = {
    RUST_BACKTRACE = "1";
    CARGO_TERM_COLOR = "always";
  };
}
