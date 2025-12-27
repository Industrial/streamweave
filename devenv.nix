{pkgs, ...}: {
  # Name of the project with version
  name = "streamweave";

  # Languages
  languages = {
    javascript = {
      enable = true;
      bun = {
        enable = true;
      };
    };

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
      targets = [];
    };
  };

  services = {
    kafka = {
      enable = true;
    };
  };

  env = {
    RUST_BACKTRACE = "1";
    CARGO_TERM_COLOR = "always";
  };

  # Development packages
  packages = with pkgs; [
    # Rust tools
    clippy
    rust-analyzer
    rustc
    rustfmt

    # Development tools
    direnv
    pre-commit

    # Formatting tools
    alejandra

    # Publishing tools
    cargo-watch
    # cargo-audit
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

    # treefmt
    actionlint
    alejandra
    beautysh
    biome
    deadnix
    rustfmt
    taplo
    treefmt
    vulnix
    yamlfmt
  ];

  # Pre-commit hooks
  git-hooks = {
    hooks = {
      commitizen = {
        enable = true;
        stages = ["commit-msg"];
      };
      pre-commit = {
        enable = true;
        name = "pre-commit";
        description = "Pre commit script, running all tasks in series";
        entry = "bin/pre-commit";
        language = "system";
        pass_filenames = false;
      };
    };
  };
}
