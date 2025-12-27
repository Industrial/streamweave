#!/usr/bin/env bash
# Workspace utilities - provides information about the workspace

set -e

case "${1:-}" in
    list)
        echo "Listing all workspace packages:"
        find packages -name Cargo.toml -type f | sed 's|packages/||; s|/Cargo.toml||' | sort
        ;;
    count)
        echo "Total packages: $(find packages -name Cargo.toml -type f | wc -l)"
        ;;
    members)
        echo "Workspace members from Cargo.toml:"
        grep -A 100 '^members = \[' Cargo.toml | grep '^    "' | sed 's/.*"\(.*\)".*/\1/' | sed 's|packages/||'
        ;;
    *)
        echo "Usage: $0 {list|count|members}"
        echo ""
        echo "Commands:"
        echo "  list     - List all packages in the workspace"
        echo "  count    - Count total number of packages"
        echo "  members  - Show workspace members from Cargo.toml"
        exit 1
        ;;
esac

