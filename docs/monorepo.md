# StreamWeave Monorepo Architecture Design

## 1. Monorepo Structure

### Directory Layout

```
streamweave/
├── Cargo.toml                    # Workspace root manifest
├── Cargo.lock                    # Workspace lock file
├── README.md
├── LICENSE
├── .github/
│   └── workflows/
│       └── ci.yml                # Updated CI for workspace
├── bin/                          # Build scripts (updated for workspace)
│   ├── build                     # Build all or specific packages
│   ├── test                      # Test all or specific packages
│   ├── check                     # Check all packages
│   ├── lint                      # Lint all packages
│   ├── format                    # Format all packages
│   ├── docs                      # Generate docs for all packages
│   ├── publish                   # Publish packages (with dependency order)
│   └── version                   # Version management across packages
├── packages/                     # All packages live here
│   ├── core/                     # Core traits and types
│   ├── pipeline/                 # Pipeline API
│   ├── graph/                    # Graph API
│   ├── message/                  # Message types
│   ├── error/                    # Error handling
│   ├── window/                   # Windowing operations
│   ├── stateful/                 # Stateful transformer support
│   ├── transaction/              # Transaction management
│   ├── offset/                   # Offset tracking
│   ├── metrics/                  # Metrics collection
│   ├── visualization/            # Visualization tools
│   ├── producers/                # Producer implementations
│   │   ├── array/
│   │   ├── vec/
│   │   ├── range/
│   │   ├── file/
│   │   ├── csv/
│   │   ├── jsonl/
│   │   ├── parquet/
│   │   ├── kafka/
│   │   ├── redis/
│   │   ├── database/
│   │   ├── http-poll/
│   │   ├── env/
│   │   └── command/
│   ├── consumers/                # Consumer implementations
│   │   ├── array/
│   │   ├── vec/
│   │   ├── file/
│   │   ├── csv/
│   │   ├── jsonl/
│   │   ├── parquet/
│   │   ├── kafka/
│   │   ├── redis/
│   │   ├── database/
│   │   ├── console/
│   │   ├── channel/
│   │   └── command/
│   ├── transformers/             # Transformer implementations
│   │   ├── map/
│   │   ├── filter/
│   │   ├── batch/
│   │   ├── reduce/
│   │   ├── merge/
│   │   ├── split/
│   │   ├── retry/
│   │   ├── rate-limit/
│   │   ├── circuit-breaker/
│   │   ├── window/
│   │   ├── group-by/
│   │   ├── sort/
│   │   ├── take/
│   │   ├── skip/
│   │   ├── sample/
│   │   ├── router/
│   │   ├── round-robin/
│   │   ├── ordered-merge/
│   │   ├── message-dedupe/
│   │   ├── interleave/
│   │   ├── partition/
│   │   ├── split-at/
│   │   ├── timeout/
│   │   ├── zip/
│   │   ├── limit/
│   │   ├── running-sum/
│   │   ├── moving-average/
│   │   └── ml/                    # ML transformers (inference, etc.)
│   ├── integrations/             # Integration packages
│   │   ├── http-server/          # HTTP server integration
│   │   ├── sql/                  # SQL query support
│   │   └── opentelemetry/        # OpenTelemetry integration
│   └── streamweave/              # Meta-package (re-exports everything)
├── examples/                     # Examples (can reference packages)
│   └── ...
├── tests/                        # Integration tests
│   └── ...
├── docs/                         # Documentation
│   └── ...
```

### Root Cargo.toml (Workspace)

```toml
[workspace]
members = [
    "packages/core",
    "packages/pipeline",
    "packages/graph",
    "packages/message",
    "packages/error",
    "packages/window",
    "packages/stateful",
    "packages/transaction",
    "packages/offset",
    "packages/metrics",
    "packages/visualization",
    # ... all producer packages
    # ... all consumer packages
    # ... all transformer packages
    # ... all integration packages
    "packages/streamweave",  # Meta-package last
]

resolver = "2"

[workspace.package]
version = "0.7.0"  # Shared version
edition = "2024"
authors = ["Tom Wieland <tom.wieland@gmail.com>"]
license = "CC-BY-SA-4.0"
repository = "https://github.com/Industrial/streamweave"
homepage = "https://github.com/Industrial/streamweave"
documentation = "https://docs.rs/streamweave"
keywords = ["stream", "async", "pipeline", "data-processing", "rust"]
categories = ["asynchronous", "data-structures"]

[workspace.dependencies]
# Shared dependencies - versions managed here
tokio = { version = "1.0", default-features = false }
futures = "0.3"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2.0.17"
tracing = "0.1.40"

# Optional dependencies (for feature flags)
csv = { version = "1.3", optional = true }
rdkafka = { version = "0.36", optional = true }
redis = { version = "0.26", optional = true }
sqlx = { version = "0.8", optional = true }
reqwest = { version = "0.12", optional = true }
axum = { version = "0.7", optional = true }
sqlparser = { version = "0.40", optional = true }
# ... etc

[workspace.lints.rust]
# Shared lint configuration
```

---

## 2. Updated bin/* Scripts

### bin/build

```bash
#!/usr/bin/env bash
# Build all packages or specific package(s)

PACKAGES="${@:-all}"

if [ "$PACKAGES" = "all" ]; then
    devenv shell -- cargo build --workspace
else
    for pkg in $PACKAGES; do
        devenv shell -- cargo build -p "streamweave-$pkg"
    done
fi
```

### bin/test

```bash
#!/usr/bin/env bash
# Test all packages or specific package(s)

PACKAGES="${@:-all}"

if [ "$PACKAGES" = "all" ]; then
    devenv shell -- cargo test --workspace
else
    for pkg in $PACKAGES; do
        devenv shell -- cargo test -p "streamweave-$pkg"
    done
fi
```

### bin/check

```bash
#!/usr/bin/env bash
# Check all packages with dependency order

devenv shell -- cargo check --workspace
```

### bin/lint

```bash
#!/usr/bin/env bash
# Lint all packages

devenv shell -- cargo clippy --workspace -- -D warnings
```

### bin/format

```bash
#!/usr/bin/env bash
# Format all packages

devenv shell -- cargo fmt --all
```

### bin/docs

```bash
#!/usr/bin/env bash
# Generate docs for all packages

devenv shell -- cargo doc --workspace --no-deps
```

### bin/publish

```bash
#!/usr/bin/env bash
# Publish packages in dependency order

# Calculate publish order based on dependencies
# Calculate publish order inline
ORDER=$(find packages -name Cargo.toml -type f | while read -r toml; do
    dir=$(dirname "$toml")
    rel_path=${dir#packages/}
    if [[ "$rel_path" == *"/"* ]]; then
        category=$(echo "$rel_path" | cut -d'/' -f1)
        name=$(echo "$rel_path" | cut -d'/' -f2)
        echo "${category}-${name}"
    else
        echo "$rel_path"
    fi
done | sort)

for pkg in $ORDER; do
    echo "Publishing streamweave-$pkg..."
    devenv shell -- cargo publish -p "streamweave-$pkg"
    sleep 5  # Rate limiting
done
```

### bin/version

```bash
#!/usr/bin/env bash
# Bump version across all packages

VERSION="$1"

if [ -z "$VERSION" ]; then
    echo "Usage: bin/version <new-version>"
    exit 1
fi

# Update all Cargo.toml files
find packages -name Cargo.toml -exec sed -i "s/^version = .*/version = \"$VERSION\"/" {} \;
```

---

## 3. Package Breakdown

### Core Packages (Required Base)

#### `streamweave-core`
- Purpose: Foundation traits and types
- Contents:
  - `Producer` trait
  - `Consumer` trait
  - `Transformer` trait
  - `Input`/`Output` traits
  - `PortList` trait (for graph API)
  - Basic configuration types
- Dependencies: Minimal (tokio, futures, async-trait)
- Features: None (always minimal)

#### `streamweave-pipeline`
- Purpose: Pipeline builder and execution
- Contents:
  - `PipelineBuilder`
  - `Pipeline` struct
  - Pipeline execution logic
- Dependencies: `streamweave-core`
- Features: None

#### `streamweave-graph`
- Purpose: Graph API for complex topologies
- Contents:
  - `Graph`, `GraphBuilder`
  - `GraphExecutor`
  - Node types (`ProducerNode`, `TransformerNode`, `ConsumerNode`)
  - Router traits and implementations
  - Connection management
- Dependencies: `streamweave-core`, `streamweave-pipeline`
- Features: None

### Optional Core Components

#### `streamweave-message`
- Purpose: Message envelope types
- Contents:
  - `Message<T>`
  - `MessageId`
  - `MessageMetadata`
  - ID generators
- Dependencies: `streamweave-core` (optional)
- Features: None

#### `streamweave-error`
- Purpose: Error handling system
- Contents:
  - `StreamError`
  - `ErrorStrategy`
  - `ErrorContext`
  - `ComponentInfo`
- Dependencies: `streamweave-core` (optional)
- Features: None

#### `streamweave-window`
- Purpose: Windowing operations
- Contents:
  - Window types (Tumbling, Sliding, Count-based)
  - Window configuration
  - Window utilities
- Dependencies: `streamweave-core`, `streamweave-message` (optional)
- Features: None

#### `streamweave-stateful`
- Purpose: Stateful transformer support
- Contents:
  - `StatefulTransformer` trait
  - `StateStore` trait
  - `InMemoryStateStore`
- Dependencies: `streamweave-core`
- Features: None

#### `streamweave-transaction`
- Purpose: Transaction management
- Contents:
  - `Transaction` types
  - `TransactionManager`
  - Transaction state management
- Dependencies: `streamweave-core`, `streamweave-message` (optional)
- Features: None

#### `streamweave-offset`
- Purpose: Offset tracking
- Contents:
  - Offset types
  - Offset management
- Dependencies: `streamweave-core`
- Features: None

### Producer Packages (Independent)

Each producer is a separate package:

- `streamweave-producer-array` - Array producer
- `streamweave-producer-vec` - Vec producer
- `streamweave-producer-range` - Range producer
- `streamweave-producer-file` - File producer
- `streamweave-producer-csv` - CSV producer (depends on csv crate)
- `streamweave-producer-jsonl` - JSONL producer
- `streamweave-producer-parquet` - Parquet producer (depends on parquet crate)
- `streamweave-producer-kafka` - Kafka producer (depends on rdkafka)
- `streamweave-producer-redis` - Redis Streams producer (depends on redis)
- `streamweave-producer-database` - Database producer (depends on sqlx)
- `streamweave-producer-http-poll` - HTTP polling producer (depends on reqwest)
- `streamweave-env` - Environment variable producer
- `streamweave-producer-command` - Command execution producer

All depend on: `streamweave-core`  
Optional dependencies: `streamweave-error`, `streamweave-message`

### Consumer Packages (Independent)

Each consumer is a separate package:

- `streamweave-consumer-array` - Array consumer
- `streamweave-consumer-vec` - Vec consumer
- `streamweave-consumer-file` - File consumer
- `streamweave-consumer-csv` - CSV consumer
- `streamweave-consumer-jsonl` - JSONL consumer
- `streamweave-consumer-parquet` - Parquet consumer
- `streamweave-consumer-kafka` - Kafka consumer
- `streamweave-consumer-redis` - Redis Streams consumer
- `streamweave-consumer-database` - Database consumer
- `streamweave-consumer-console` - Console consumer
- `streamweave-consumer-tokio-channel` - Channel consumer
- `streamweave-consumer-command` - Command consumer

All depend on: `streamweave-core`  
Optional dependencies: `streamweave-error`, `streamweave-message`

### Transformer Packages (Independent)

Each transformer is a separate package:

- `streamweave-transformer-map` - Map transformer
- `streamweave-transformer-filter` - Filter transformer
- `streamweave-transformer-batch` - Batch transformer
- `streamweave-transformer-reduce` - Reduce transformer
- `streamweave-transformer-merge` - Merge transformer
- `streamweave-transformer-split` - Split transformer
- `streamweave-transformer-retry` - Retry transformer
- `streamweave-transformer-rate-limit` - Rate limit transformer
- `streamweave-transformer-circuit-breaker` - Circuit breaker transformer
- `streamweave-transformer-window` - Window transformer
- `streamweave-transformer-group-by` - Group by transformer
- `streamweave-transformer-sort` - Sort transformer
- `streamweave-transformer-take` - Take transformer
- `streamweave-transformer-skip` - Skip transformer
- `streamweave-transformer-sample` - Sample transformer
- `streamweave-transformer-router` - Router transformer
- `streamweave-transformer-round-robin` - Round-robin transformer
- `streamweave-transformer-ordered-merge` - Ordered merge transformer
- `streamweave-transformer-message-dedupe` - Message deduplication
- `streamweave-transformer-interleave` - Interleave transformer
- `streamweave-transformer-partition` - Partition transformer
- `streamweave-transformer-split-at` - Split at transformer
- `streamweave-transformer-timeout` - Timeout transformer
- `streamweave-transformer-zip` - Zip transformer
- `streamweave-transformer-limit` - Limit transformer
- `streamweave-transformer-running-sum` - Running sum transformer
- `streamweave-transformer-moving-average` - Moving average transformer
- `streamweave-transformer-ml` - ML transformers (inference, etc.)

All depend on: `streamweave-core`  
Optional dependencies: `streamweave-error`, `streamweave-message`, `streamweave-window`, `streamweave-stateful`

### Integration Packages

#### `streamweave-integration-http-server`
- Purpose: HTTP server integration
- Contents: HTTP graph server, request/response handling
- Dependencies: `streamweave-core`, `streamweave-graph`, axum, tower
- Features: None

#### `streamweave-integration-sql`
- Purpose: SQL query support
- Contents: SQL parser, query builder
- Dependencies: `streamweave-core`, sqlparser
- Features: None

#### `streamweave-integration-opentelemetry`
- Purpose: OpenTelemetry integration
- Contents: OpenTelemetry metrics/tracing
- Dependencies: `streamweave-core`, `streamweave-metrics`, opentelemetry crates
- Features: None

### Utility Packages

#### `streamweave-metrics`
- Purpose: Metrics collection
- Contents: Metrics types, collectors, Prometheus export
- Dependencies: `streamweave-core`
- Features: `prometheus`, `opentelemetry`

#### `streamweave-visualization`
- Purpose: Visualization tools
- Contents: Graph visualization, DAG export, HTML generation
- Dependencies: `streamweave-core`, `streamweave-graph`
- Features: None

### Meta-Package

#### `streamweave`
- Purpose: Convenience package that re-exports everything
- Contents: Re-exports from all packages
- Dependencies: All other packages (optional features)
- Features:
  - `default` - Includes common packages
  - `all-producers` - All producer packages
  - `all-consumers` - All consumer packages
  - `all-transformers` - All transformer packages
  - `all-integrations` - All integration packages
  - Individual feature flags for each package

---

## 4. Dependency Management Per Package

### Yes, each package has its own dependencies

Each package has its own `Cargo.toml` with:

1. Workspace dependency references (for shared versions)
2. Package-specific dependencies
3. Optional dependencies via features
4. Minimal dependency sets

### Example: `packages/producers/kafka/Cargo.toml`

```toml
[package]
name = "streamweave-producer-kafka"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
# Core dependency
streamweave-core = { path = "../../core", version.workspace = true }

# Optional core components
streamweave-error = { path = "../../error", version.workspace = true, optional = true }
streamweave-message = { path = "../../message", version.workspace = true, optional = true }

# Kafka-specific dependency
rdkafka = { workspace = true, optional = true }

# Shared dependencies
tokio = { workspace = true, features = ["full"] }
futures = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

[features]
default = ["kafka"]
kafka = ["dep:rdkafka"]
error-handling = ["dep:streamweave-error"]
message-envelope = ["dep:streamweave-message"]
```

### Example: `packages/transformers/map/Cargo.toml`

```toml
[package]
name = "streamweave-transformer-map"
version.workspace = true
edition.workspace = true

[dependencies]
streamweave-core = { path = "../../core", version.workspace = true }
streamweave-error = { path = "../../error", version.workspace = true, optional = true }

[features]
default = []
error-handling = ["dep:streamweave-error"]
```

### Dependency Graph Strategy

1. Core packages: Minimal dependencies
2. Producer/Consumer/Transformer packages: Only `streamweave-core` required
3. Integration packages: Core + specific integration dependencies
4. Meta-package: All packages as optional dependencies

### Benefits

- Users can depend only on what they need
- Faster compilation (only compile used packages)
- Smaller binary sizes
- Clear dependency boundaries
- Independent versioning (if needed later)
- Easier maintenance and testing

---

## Additional Considerations

### Version Management

- Shared version in workspace (recommended for now)
- Can move to independent versions later if needed
- Use `version.workspace = true` in each package

### Publishing Strategy

1. Publish core packages first
2. Publish optional components
3. Publish producer/consumer/transformer packages (can be parallel)
4. Publish integration packages
5. Publish meta-package last

### Testing Strategy

- Each package has its own tests
- Integration tests in `tests/` directory
- Can test packages independently
- Workspace-level test suite

### Documentation Strategy

- Each package has its own docs
- Workspace-level docs in root
- Cross-package documentation links
- Examples can reference specific packages

This structure provides maximum modularity while maintaining ease of use through the meta-package.

