#!/usr/bin/env bash
# Ensures direnv environment is loaded before running commands
# This should be sourced or executed before running any commands in Cursor

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Source the shell-init script if it exists
source "$PROJECT_ROOT/.cursor/shell-init.sh"

