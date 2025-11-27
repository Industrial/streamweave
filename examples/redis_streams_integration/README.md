# Redis Streams Integration Example

This example demonstrates how to use StreamWeave with Redis Streams for lightweight message streaming. It shows both consuming from and producing to Redis streams, with consumer groups, message acknowledgment, and comprehensive error handling.

## Prerequisites

Before running this example, you need:

1. **Redis server running** - Either:
   - Local Redis installation (version 5.0+ for Streams support)
   - Docker with Redis
   - Remote Redis instance

2. **Redis Streams feature enabled** - Build with `--features redis-streams`

## Quick Start with devenv.sh (Recommended)

If you're using `devenv.sh`, Redis is automatically configured:

```bash
# Activate devenv environment
devenv shell

# Redis should be available at localhost:6379
# Verify it's running:
redis-cli ping
# Should return: PONG
```

## Alternative: Quick Start with Docker

If not using devenv, the easiest way to run Redis locally is using Docker:

```bash
# Start Redis
docker run -d --name redis -p 6379:6379 redis:7-alpine

# Verify it's running
docker exec redis redis-cli ping
# Should return: PONG
```

## Alternative: Local Redis Installation

If you have Redis installed locally:

```bash
# Start Redis server
redis-server

# Or on macOS with Homebrew:
brew services start redis

# Verify it's running
redis-cli ping
# Should return: PONG
```

## Running Examples

### 1. Produce Messages to Redis Streams

This example creates sample events and sends them to a Redis stream:

```bash
cargo run --example redis_streams_integration --features redis-streams produce
```

**What it demonstrates:**
- Creating a `RedisStreamsConsumer` (producer in Redis Streams terms)
- Serializing events to JSON and storing in stream fields
- Error handling with retry strategy
- Stream length management (maxlen with approximate trimming)

**Expected output:**
```
🚀 StreamWeave Redis Streams Integration Example
=================================================

Running: Produce to Redis Streams
---------------------------------
📤 Setting up Redis Streams producer...
🚀 Starting pipeline to produce to Redis Streams...
   Stream: example-stream
   Max length: 1000 messages (approximate)
   Sending 5 events...

✅ All events sent successfully!
```

**Verify messages were sent:**
```bash
# Check the stream
redis-cli XRANGE example-stream - + COUNT 10
```

### 2. Consume Messages from Redis Streams

This example reads messages from a Redis stream using a consumer group:

```bash
cargo run --example redis_streams_integration --features redis-streams consume
```

**What it demonstrates:**
- Consuming from Redis Streams using XREAD/XREADGROUP
- Consumer group usage for distributed processing
- Message acknowledgment (auto-ack enabled)
- Error handling for deserialization failures
- Blocking reads with configurable timeout

**Before running, make sure you've produced some messages:**
```bash
# First, produce some messages
cargo run --example redis_streams_integration --features redis-streams produce

# Then in another terminal, consume them
cargo run --example redis_streams_integration --features redis-streams consume
```

**Expected output:**
```
🚀 StreamWeave Redis Streams Integration Example
=================================================

Running: Consume from Redis Streams
-----------------------------------
📥 Setting up Redis Streams consumer...
🚀 Starting pipeline to consume from Redis Streams...
   Stream: example-stream
   Consumer Group: streamweave-example-group
   Consumer: consumer-1
   Press Ctrl+C to stop

  ✓ Received message: stream=example-stream, id=1234567890-0, event_id=1, message=First event
  ✓ Received message: stream=example-stream, id=1234567891-0, event_id=2, message=Second event
  ...
```

**Verify consumer group status:**
```bash
# Check consumer group info
redis-cli XINFO GROUPS example-stream

# Check pending messages
redis-cli XPENDING example-stream streamweave-example-group
```

### 3. Round-Trip Example (Consume → Transform → Produce)

This example demonstrates a complete pipeline:
- Reads from `input-stream`
- Transforms messages (adds [PROCESSED] prefix)
- Writes to `output-stream`

```bash
cargo run --example redis_streams_integration --features redis-streams roundtrip
```

**Before running, populate the input stream:**
```bash
# Option 1: Use the produce example but modify it to write to input-stream
# Option 2: Manually add messages to input-stream
redis-cli XADD input-stream * data '{"id":1,"message":"Test event","timestamp":1234567890}'
```

**What it demonstrates:**
- Full pipeline: consume → transform → produce
- Consumer group offset management
- Message transformation
- Error handling across the pipeline

## Understanding Redis Streams Concepts

### Streams
Redis Streams are an append-only log data structure. Each message has:
- **ID**: Unique identifier (timestamp-sequence format)
- **Fields**: Key-value pairs containing the message data

### Consumer Groups
Consumer groups allow multiple consumers to process the same stream:
- **Group**: Named group of consumers
- **Consumer**: Individual consumer within a group
- **Pending messages**: Messages delivered but not acknowledged
- **XREADGROUP**: Command to read from a consumer group

### Message Acknowledgment
- Messages read via XREADGROUP are marked as "pending"
- **XACK**: Acknowledges a message, removing it from pending list
- **Auto-ack**: Automatically acknowledge messages after reading
- **Manual ack**: Acknowledge messages after processing

## Configuration Options

### Producer Configuration (RedisStreamsProducerConfig)

```rust
RedisStreamsConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")  // Redis connection URL
    .with_stream("my-stream")                       // Stream name
    .with_group("my-group")                         // Consumer group (optional)
    .with_consumer("consumer-1")                    // Consumer name (required if using groups)
    .with_start_id(">")                             // Start ID: "0" for beginning, ">" for new, "$" for latest
    .with_block_ms(1000)                            // Block time in milliseconds (0 = non-blocking)
    .with_count(10)                                 // Messages per read (optional)
    .with_auto_ack(true)                            // Auto-acknowledge messages
```

### Consumer Configuration (RedisStreamsProducerConfig)

```rust
RedisStreamsProducerConfig::default()
    .with_connection_url("redis://localhost:6379")  // Redis connection URL
    .with_stream("my-stream")                       // Stream name
    .with_maxlen(1000)                              // Maximum stream length (trims old messages)
    .with_approximate_maxlen(true)                   // Use approximate trimming (more efficient)
```

## Common Use Cases

### 1. Event Processing Pipeline
- Producer: External systems write events to stream
- Consumer: StreamWeave processes events through pipeline
- Transformation: Enrich, filter, or aggregate events
- Output: Write processed events to another stream or database

### 2. Distributed Task Queue
- Producer: Tasks added to stream
- Consumer Group: Multiple workers process tasks
- Acknowledgment: Tasks acknowledged after completion
- Pending List: Monitor unprocessed tasks

### 3. Real-time Data Processing
- Producer: High-frequency data source (sensors, logs, etc.)
- Consumer: Process and aggregate data
- Windowing: Use StreamWeave window transformers
- Output: Write aggregated results to another system

## Troubleshooting

### Connection Errors

**Error**: `Failed to create Redis client` or `Failed to connect to Redis`

**Solutions**:
1. Verify Redis is running: `redis-cli ping`
2. Check connection URL: Default is `redis://localhost:6379`
3. Check Redis is accessible: `redis-cli -h localhost -p 6379 ping`
4. For Docker: Ensure port 6379 is exposed

### Consumer Group Errors

**Error**: `NOGROUP` or consumer group not found

**Solutions**:
1. Consumer groups are created automatically on first read
2. If stream doesn't exist, create it first: `redis-cli XADD my-stream * field value`
3. Use `XGROUP CREATE` manually if needed:
   ```bash
   redis-cli XGROUP CREATE my-stream my-group 0
   ```

### No Messages Received

**Symptoms**: Consumer runs but receives no messages

**Solutions**:
1. Verify messages exist: `redis-cli XRANGE my-stream - +`
2. Check start_id: Use `"0"` to read from beginning, `">"` for new messages only
3. Verify consumer group: Check with `XINFO GROUPS my-stream`
4. Check pending messages: `redis-cli XPENDING my-stream my-group`

### Deserialization Errors

**Error**: `Failed to deserialize message`

**Solutions**:
1. Ensure message format matches expected structure
2. Check field names: Example expects `data` field with JSON
3. Verify JSON is valid: `redis-cli XRANGE my-stream - +`
4. Use error strategy `Skip` to continue processing other messages

### Performance Issues

**Symptoms**: Slow processing or high memory usage

**Solutions**:
1. Use `maxlen` to limit stream size: `.with_maxlen(1000)`
2. Use approximate trimming: `.with_approximate_maxlen(true)`
3. Adjust `count` parameter: Read more messages per call
4. Use consumer groups for parallel processing
5. Monitor with `XINFO STREAM my-stream`

## Advanced Examples

### Reading from Beginning

To read all messages from the start of the stream:

```rust
.with_start_id("0")  // Read from beginning
```

### Reading Only New Messages

To read only new messages (after consumer starts):

```rust
.with_start_id(">")  // Read new messages in consumer group
```

### Manual Acknowledgment

To manually acknowledge messages after processing:

```rust
.with_auto_ack(false)  // Disable auto-ack

// Then acknowledge manually in your transformer/consumer
// (This requires accessing the Redis connection directly)
```

### Monitoring Pending Messages

Check for unprocessed messages:

```bash
# List pending messages
redis-cli XPENDING my-stream my-group

# Claim and process pending messages
redis-cli XCLAIM my-stream my-group consumer-2 0 1234567890-0
```

## Integration with Other StreamWeave Features

### Error Handling

```rust
.with_error_strategy(ErrorStrategy::Retry(3))  // Retry 3 times
.with_error_strategy(ErrorStrategy::Skip)      // Skip errors
.with_error_strategy(ErrorStrategy::Stop)      // Stop on error
```

### Transformers

Use StreamWeave transformers to process Redis Streams messages:

```rust
// Map transformer to transform messages
let transformer = MapTransformer::new(|msg: RedisStreamsMessage| {
    // Transform message
});

// Batch transformer to process in batches
let batch = BatchTransformer::new(100);

// Filter transformer to filter messages
let filter = FilterTransformer::new(|msg: &RedisStreamsMessage| {
    // Filter condition
});
```

### Stateful Processing

Use stateful transformers for aggregations:

```rust
use streamweave::stateful_transformer::{StatefulTransformer, InMemoryStateStore};

// Aggregate events by some criteria
// (See stateful transformer examples)
```

## Further Reading

- [Redis Streams Documentation](https://redis.io/docs/data-types/streams/)
- [Redis Streams Tutorial](https://redis.io/docs/data-types/streams-tutorial/)
- [StreamWeave Documentation](https://docs.rs/streamweave)
- [StreamWeave Examples](../README.md)

## See Also

- [Kafka Integration Example](../kafka_integration/README.md) - Similar example using Kafka
- [Basic Pipeline Example](../basic_pipeline/README.md) - Introduction to StreamWeave
- [Advanced Pipeline Example](../advanced_pipeline/README.md) - Complex pipeline patterns

