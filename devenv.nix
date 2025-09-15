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
    gitAndTools.gh # GitHub CLI
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
        entry = "cargo build";
        pass_filenames = false;
      };

      # Test
      test = {
        enable = true;
        name = "cargo-test";
        description = "Run cargo tests";
        entry = "cargo test";
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

  # Automatic commands
  enterShell = ''
    echo "🦀 StreamWeave Development Environment"
    echo "📦 Available commands:"
    echo "  cargo build     - Build the project"
    echo "  cargo test      - Run tests"
    echo "  cargo clippy    - Run linter"
    echo "  cargo fmt       - Format code"
    echo "  cargo audit     - Security audit"
    echo "  cargo doc       - Generate documentation"
    echo "  cargo publish   - Publish to crates.io"
    echo ""
    echo "🔍 Running initial checks..."
    cargo check
    cargo clippy -- -D warnings
  '';

  processes = {
    cargo-watch = {
      exec = "cargo watch -x check -x test";
    };
  };

  # Environment variables
  env = {
    RUST_BACKTRACE = "1";
    CARGO_TERM_COLOR = "always";
  };
}
