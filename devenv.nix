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
  ];
}
