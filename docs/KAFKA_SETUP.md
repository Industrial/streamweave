# Kafka Integration Setup

This document describes how to set up the Kafka integration feature for StreamWeave.

## Prerequisites

The Kafka integration requires the following system dependencies:

- **cmake** - For building librdkafka
- **pkg-config** - For finding system libraries
- **OpenSSL** - For SSL/TLS support
- **zlib** - Compression library
- **zstd** - Compression library

## Development Setup (devenv)

If you're using `devenv.sh`, these dependencies are automatically provided in `devenv.nix`.

To activate the devenv environment:

```bash
# If using direnv (automatic)
direnv allow

# Or manually activate
devenv shell
```

After activating, verify the dependencies are available:

```bash
which cmake
which pkg-config
cmake --version
```

## Building with Kafka Feature

Once the environment is set up, you can build StreamWeave with Kafka support:

```bash
cargo build --features kafka
```

Or check if it compiles:

```bash
cargo check --features kafka
```

## Usage

### Kafka Producer (consumes from Kafka)

```rust
use streamweave::producers::kafka::{KafkaProducer, KafkaConsumerConfig};
use streamweave::pipeline::Pipeline;

let producer = KafkaProducer::new(
    KafkaConsumerConfig::default()
        .with_bootstrap_servers("localhost:9092")
        .with_group_id("my-consumer-group")
        .with_topic("my-topic")
);

// Use in a pipeline...
```

### Kafka Consumer (produces to Kafka)

```rust
use streamweave::consumers::kafka::{KafkaConsumer, KafkaProducerConfig};
use streamweave::pipeline::Pipeline;

let consumer = KafkaConsumer::new(
    KafkaProducerConfig::default()
        .with_bootstrap_servers("localhost:9092")
        .with_topic("my-topic")
);

// Use in a pipeline...
```

## Troubleshooting

### Build Errors

**Error: "cmake not found"**
- Solution: Ensure devenv is activated (`devenv shell` or `direnv allow`)

**Error: "OpenSSL not found"**
- Solution: Ensure devenv is activated and OpenSSL is in the environment

**Error: "librdkafka build failed"**
- Solution: Check that all prerequisites (cmake, pkg-config, openssl, zlib, zstd) are available
- Verify by running: `cmake --version && pkg-config --version`

### Runtime Errors

**Connection errors to Kafka brokers**
- Verify Kafka brokers are running and accessible
- Check bootstrap server addresses are correct
- Ensure network connectivity

**Consumer group errors**
- Verify the consumer group ID is valid
- Check for conflicting consumer group instances

## CI/CD Considerations

For CI/CD environments, ensure that the build system has:

1. cmake installed
2. pkg-config installed
3. OpenSSL development libraries
4. zlib development libraries
5. zstd development libraries

Or use a container/image that includes these dependencies (like the Nix-based devenv environment).

