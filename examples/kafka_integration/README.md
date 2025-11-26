# Kafka Integration Example

This example demonstrates how to use StreamWeave with Apache Kafka for message streaming. It shows both consuming from and producing to Kafka topics, with comprehensive error handling and configuration options.

## Prerequisites

Before running this example, you need:

1. **Kafka cluster running** - Either:
   - Local Kafka installation
   - Docker with Kafka
   - Remote Kafka cluster

2. **Build dependencies** (provided by devenv if using `devenv.sh`):
   - cmake
   - pkg-config
   - OpenSSL
   - zlib
   - zstd

3. **Kafka feature enabled** - Build with `--features kafka`

## Quick Start with Docker

The easiest way to run Kafka locally is using Docker Compose:

```bash
# Start Kafka and Zookeeper
docker run -d --name zookeeper -p 2181:2181 confluentinc/cp-zookeeper:latest
docker run -d --name kafka -p 9092:9092 \
  -e KAFKA_BROKER_ID=1 \
  -e KAFKA_ZOOKEEPER_CONNECT=localhost:2181 \
  -e KAFKA_ADVERTISED_LISTENERS=PLAINTEXT://localhost:9092 \
  -e KAFKA_OFFSETS_TOPIC_REPLICATION_FACTOR=1 \
  confluentinc/cp-kafka:latest

# Or use docker-compose (create docker-compose.yml):
```

Alternatively, use the official Kafka quickstart:
```bash
# Download Kafka from https://kafka.apache.org/downloads
# Extract and start:
bin/zookeeper-server-start.sh config/zookeeper.properties
bin/kafka-server-start.sh config/server.properties
```

## Creating Topics

Before running the examples, create the required topics:

```bash
# Using kafka-console scripts (if Kafka is installed locally)
kafka-topics.sh --create --topic example-topic --bootstrap-server localhost:9092 --partitions 3 --replication-factor 1

# For round-trip example
kafka-topics.sh --create --topic input-topic --bootstrap-server localhost:9092 --partitions 3 --replication-factor 1
kafka-topics.sh --create --topic output-topic --bootstrap-server localhost:9092 --partitions 3 --replication-factor 1

# Using Docker
docker exec kafka kafka-topics --create --topic example-topic --bootstrap-server localhost:9092 --partitions 3 --replication-factor 1
```

## Running Examples

### 1. Produce Messages to Kafka

This example creates sample events and sends them to a Kafka topic with batching and compression:

```bash
cargo run --example kafka_integration --features kafka produce
```

**What it demonstrates:**
- Creating a `KafkaConsumer` (producer in Kafka terms) with batching configuration
- Serializing events to JSON
- Error handling with retry strategy
- Producer batching (16KB batches with 10ms linger)
- Message compression (gzip)

### 2. Consume Messages from Kafka

This example reads messages from a Kafka topic using a consumer group:

```bash
cargo run --example kafka_integration --features kafka consume
```

**What it demonstrates:**
- Creating a `KafkaProducer` (consumer in Kafka terms) with consumer group
- Reading from Kafka topics with partition assignment
- Deserializing JSON messages
- Offset management (auto-commit enabled)
- Error handling (skip malformed messages)

### 3. Round-trip Processing

This example reads from one topic, transforms messages, and writes to another topic:

```bash
cargo run --example kafka_integration --features kafka roundtrip
```

**What it demonstrates:**
- Full pipeline: consume → transform → produce
- Consumer group offset tracking
- Message transformation in the pipeline
- Separate input/output topic configuration

## Example Output

### Produce Example
```
🚀 StreamWeave Kafka Integration Example
==========================================

Running: Produce to Kafka
-------------------------
📤 Setting up Kafka producer...
🚀 Starting pipeline to produce to Kafka...
   Topic: example-topic
   Batch size: 16KB
   Compression: gzip
   Sending 5 events...

✅ All events sent successfully!
```

### Consume Example
```
🚀 StreamWeave Kafka Integration Example
==========================================

Running: Consume from Kafka
---------------------------
📥 Setting up Kafka consumer...
🚀 Starting pipeline to consume from Kafka...
   Topic: example-topic
   Consumer Group: streamweave-example-group
   Press Ctrl+C to stop

  ✓ Received message: topic=example-topic, partition=0, offset=0, id=1, message=First event
  ✓ Received message: topic=example-topic, partition=0, offset=1, id=2, message=Second event
  ...
```

## Key Concepts Demonstrated

### Consumer Groups

Consumer groups enable parallel processing and automatic load balancing:

```rust
KafkaConsumerConfig::default()
    .with_group_id("my-consumer-group")  // All consumers in this group share partitions
    .with_topic("example-topic")
```

- Multiple consumers with the same `group_id` will automatically share partitions
- Each partition is consumed by only one consumer in the group
- Offset is tracked per partition per consumer group

### Offset Management

Offsets track the last processed message position:

```rust
.with_auto_offset_reset("earliest")  // Start from beginning on new group
.with_enable_auto_commit(true)       // Automatically commit offsets
.with_auto_commit_interval_ms(5000)  // Commit every 5 seconds
```

- `"earliest"` - Start from the beginning when no offset exists
- `"latest"` - Start from the end (only new messages)
- Auto-commit ensures progress is tracked even if the consumer crashes

### Producer Batching

Batching improves throughput by grouping multiple messages:

```rust
.with_batch_size(16384)    // 16KB batch size
.with_linger_ms(10)        // Wait 10ms to fill batch
.with_compression_type("gzip")  // Compress batches
```

- Messages are accumulated until batch size is reached or linger time expires
- Larger batches = better throughput but higher latency
- Compression reduces network usage

### Error Handling

StreamWeave provides flexible error handling:

```rust
.with_error_strategy(ErrorStrategy::Skip)      // Skip failed messages
.with_error_strategy(ErrorStrategy::Retry(3))  // Retry up to 3 times
.with_error_strategy(ErrorStrategy::Stop)      // Stop on first error
```

- Use `Skip` for non-critical messages
- Use `Retry` for transient failures (network issues)
- Use `Stop` when data integrity is critical

## Troubleshooting

### Connection Errors

**Error: "Failed to connect to Kafka broker"**
- Verify Kafka is running: `docker ps` or check Kafka logs
- Check bootstrap servers address: Default is `localhost:9092`
- Ensure network connectivity

### Topic Not Found

**Error: "Topic does not exist"**
- Create the topic before running: `kafka-topics --create ...`
- Check topic name matches configuration
- Verify topic exists: `kafka-topics --list --bootstrap-server localhost:9092`

### Consumer Group Issues

**Error: "Consumer group rebalance failed"**
- Ensure consumer group ID is unique for your use case
- Check for multiple consumers with same group ID causing conflicts
- Increase `max_poll_interval_ms` if processing takes time

### Build Errors

**Error: "cmake not found" or "OpenSSL not found"**
- Activate devenv: `devenv shell` or `direnv allow`
- Verify dependencies: `which cmake && which pkg-config`
- See [KAFKA_SETUP.md](../../docs/KAFKA_SETUP.md) for details

## Advanced Configuration

### Manual Offset Management

For more control, disable auto-commit and manage offsets manually:

```rust
.with_enable_auto_commit(false)
```

### Custom Kafka Properties

Add any Kafka configuration property:

```rust
.with_custom_property("max.poll.records", "500")
.with_custom_property("fetch.min.bytes", "1024")
```

### Partition Assignment

Kafka automatically assigns partitions to consumers in a group. For explicit control, you can use custom partition assignment strategies via custom properties.

## Next Steps

- Try the Redis Streams example for a lighter-weight alternative
- Explore database integration examples for query result streaming
- Check out the advanced transformers example for data processing patterns

