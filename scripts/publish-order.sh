#!/usr/bin/env bash
# Calculate publish order based on dependencies
# This script analyzes the dependency graph and returns packages in topological order

# Get all package directories
find packages -name Cargo.toml -type f | while read -r toml; do
    # Extract package name from path
    # packages/core/Cargo.toml -> core
    # packages/producers/kafka/Cargo.toml -> producer-kafka
    dir=$(dirname "$toml")
    rel_path=${dir#packages/}

    # Convert path to package name format
    # core -> core
    # producers/kafka -> producer-kafka
    if [[ "$rel_path" == *"/"* ]]; then
        # Has subdirectory (producers/kafka)
        category=$(echo "$rel_path" | cut -d'/' -f1)
        name=$(echo "$rel_path" | cut -d'/' -f2)
        # Convert kebab-case to match package naming
        echo "${category}-${name}"
    else
        # Top-level package (core, pipeline, etc.)
        echo "$rel_path"
    fi
done | sort

# Note: This is a simplified version. A full implementation would:
# 1. Parse all Cargo.toml files to extract dependencies
# 2. Build a dependency graph
# 3. Perform topological sort
# 4. Return packages in dependency order (core packages first, then dependents)

