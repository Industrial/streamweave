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
        # "llvm-tools-preview"
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

    # Added from the code block
    cargo-watch

    # # LLVM tools for coverage testing
    # llvmPackages_18.llvm
    # llvmPackages_18.libclang
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
    };
  };

  # Automatic commands
  enterShell = ''
    echo "🦀 Running initial cargo build..."
    cargo build
  '';

  processes = {
    cargo-watch = {
      exec = "cargo watch -x check -x test";
    };
  };
}
