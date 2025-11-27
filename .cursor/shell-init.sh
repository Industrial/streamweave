#!/usr/bin/env bash
# Shell initialization script to load direnv environment for all commands
# This ensures devenv environment is always available in Cursor terminals

# Only run if we're in the project directory
if [ -f "$(dirname "${BASH_SOURCE[0]}")/../.envrc" ]; then
  PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  
  # Load direnv if available and .envrc exists
  if command -v direnv >/dev/null 2>&1 && [ -f "$PROJECT_ROOT/.envrc" ]; then
    # Allow direnv to load (non-interactive)
    export DIRENV_LOG_FORMAT=""
    cd "$PROJECT_ROOT" && direnv allow >/dev/null 2>&1 || true
    
    # Export direnv environment variables
    eval "$(cd "$PROJECT_ROOT" && direnv export bash)" 2>/dev/null || true
  fi
fi

