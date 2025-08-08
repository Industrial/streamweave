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
      ];
      targets = [
        "wasm32-unknown-unknown"
      ];
    };
  };

  # Development packages
  packages = with pkgs; [
    # Rust tools
    cargo-nextest
    clippy
    rust-analyzer
    rustfmt
    sea-orm-cli
    wasm-bindgen-cli
    wasm-pack

    # Development tools
    direnv
    pre-commit

    # Formatting tools
    alejandra

    # Added from the code block
    cargo-watch

    # TaskMaster AI dependencies
    bun
  ];

  # Pre-commit hooks
  pre-commit.hooks = {
    # Rust checks
    cargo-check.enable = true;
    clippy.enable = true;
    rustfmt.enable = true;

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
      entry = "cargo test --workspace";
      pass_filenames = false;
    };
  };

  # Automatic commands
  enterShell = ''
    echo "🦀 Running initial cargo build..."
    cargo build

    echo "🤖 Setting up TaskMaster AI with Bun..."
    if [ ! -d ".taskmaster" ]; then
      echo "Initializing TaskMaster AI..."
      bunx taskmaster-ai init --yes
    fi

    echo "✅ Development environment ready!"
    echo "📋 Available commands:"
    echo "  - bunx taskmaster-ai tasks          # List all tasks"
    echo "  - bunx taskmaster-ai next           # Get next task to work on"
    echo "  - bunx taskmaster-ai add 'task'     # Add new task"
    echo "  - bunx taskmaster-ai research 'query' # Research with AI"
  '';

  processes = {
    cargo-watch.exec = "cargo watch -x check -x test";
  };
}
