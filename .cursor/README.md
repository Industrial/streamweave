# Cursor Configuration

This directory contains Cursor-specific configuration files.

## Shell Environment Setup

The `shell-init.sh` script ensures that direnv is loaded for all shell commands executed in Cursor terminals. This ensures that the devenv environment (including OpenSSL, cmake, and other build dependencies) is always available.

### Automatic Loading for AI Assistant

**For the AI assistant to automatically load direnv for all commands:**

All terminal commands executed by the AI assistant should first source this script. The script is located at:
```
.cursor/shell-init.sh
```

When running commands in the terminal, ensure direnv is loaded by prefixing commands with:
```bash
source .cursor/shell-init.sh && <your-command>
```

Or, for simplicity, the assistant can use the wrapper:
```bash
source .cursor/ensure-direnv.sh && <your-command>
```

### Manual Loading

You can manually source the script in your terminal:
```bash
source .cursor/shell-init.sh
```

### How It Works

The `shell-init.sh` script:
- Detects the project root directory
- Checks if direnv is available
- Allows direnv to load the `.envrc` file (non-interactively)
- Exports all environment variables set by direnv/devenv

This ensures that when running commands like `bin/check`, `bin/build`, etc., all required dependencies (OpenSSL, cmake, pkg-config, etc.) are available in the PATH and environment.

### Files

- `shell-init.sh` - Main initialization script that loads direnv
- `ensure-direnv.sh` - Wrapper script that sources shell-init.sh
- `README.md` - This file

### Verification

After loading, verify that dependencies are available:
```bash
which cmake
which pkg-config
pkg-config --exists openssl && echo "OpenSSL found"
```

