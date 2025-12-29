# streamweave-redis

[![Crates.io](https://img.shields.io/crates/v/streamweave-redis.svg)](https://crates.io/crates/streamweave-redis)
[![Documentation](https://docs.rs/streamweave-redis/badge.svg)](https://docs.rs/streamweave-redis)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Redis Streams integration for StreamWeave**  
*Produce to and consume from Redis Streams with streaming processing.*

The `streamweave-redis` package provides Redis Streams producers and consumers for StreamWeave. It enables reading from Redis Streams and writing to Redis Streams with consumer groups, message acknowledgment, and stream length management.

## ✨ Key Features

- **RedisProducer**: Consume messages from Redis Streams and stream them
- **RedisConsumer**: Produce messages to Redis Streams from streams
- **Consumer Groups**: Support for Redis Streams consumer groups
- **Message Acknowledgment**: Automatic and manual message acknowledgment
- **Stream Length Management**: Configurable stream length limits
- **XADD/XREAD Operations**: Direct Redis Streams command support
- **Error Handling**: Comprehensive error handling with retry strategies

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-redis = { version = "0.3.0", features = ["redis"] }
```

## 🚀 Quick Start

### Consume from Redis Streams

```rust
use streamweave_redis::producers::{RedisProducer, RedisConsumerConfig};
use streamweave_pipeline::PipelineBuilder;

let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("mystream")
    .with_group("my-group")
    .with_consumer("consumer-1")
    .with_start_id("0");

let pipeline = PipelineBuilder::new()
    .producer(RedisProducer::new(config))
    .consumer(/* process messages */);

pipeline.run().await?;
```

### Produce to Redis Streams

```rust
use streamweave_redis::consumers::{RedisConsumer, RedisProducerConfig};
use streamweave_pipeline::PipelineBuilder;
use serde::Serialize;

#[derive(Serialize)]
struct Event {
    id: u32,
    message: String,
}

let config = RedisProducerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("mystream")
    .with_maxlen(10000);

let pipeline = PipelineBuilder::new()
    .producer(/* produce events */)
    .consumer(RedisConsumer::<Event>::new(config));

pipeline.run().await?;
```

## 📖 API Overview

### RedisProducer

Consumes messages from Redis Streams and streams them:

```rust
pub struct RedisProducer {
    pub config: ProducerConfig<RedisMessage>,
    pub redis_config: RedisConsumerConfig,
}
```

**Key Methods:**
- `new(config)` - Create producer with Redis configuration
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from Redis Streams messages

### RedisConsumer

Produces messages to Redis Streams from streams:

```rust
pub struct RedisConsumer<T> {
    pub config: ConsumerConfig<T>,
    pub redis_config: RedisProducerConfig,
}
```

**Key Methods:**
- `new(config)` - Create consumer with Redis configuration
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Send stream items to Redis Stream

### RedisMessage

Represents a message received from Redis Streams:

```rust
pub struct RedisMessage {
    pub stream: String,
    pub id: String,
    pub fields: HashMap<String, String>,
}
```

## 📚 Usage Examples

### Consumer Group Setup

Configure consumer groups for distributed processing:

```rust
use streamweave_redis::producers::{RedisProducer, RedisConsumerConfig};

let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("events")
    .with_group("my-consumer-group")
    .with_consumer("consumer-1")
    .with_start_id(">")  // Read new messages only
    .with_block_ms(5000)
    .with_count(100)
    .with_auto_ack(true);

let producer = RedisProducer::new(config);
```

### Reading from Beginning

Read all messages from the beginning of a stream:

```rust
use streamweave_redis::producers::{RedisProducer, RedisConsumerConfig};

let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("events")
    .with_start_id("0")  // Start from beginning
    .with_block_ms(1000);

let producer = RedisProducer::new(config);
```

### Reading New Messages Only

Read only new messages (after current position):

```rust
use streamweave_redis::producers::{RedisProducer, RedisConsumerConfig};

let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("events")
    .with_start_id("$")  // Read only new messages
    .with_block_ms(5000);

let producer = RedisProducer::new(config);
```

### Stream Length Management

Limit stream length to prevent unbounded growth:

```rust
use streamweave_redis::consumers::{RedisConsumer, RedisProducerConfig};

let config = RedisProducerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("events")
    .with_maxlen(10000)  // Keep only last 10000 messages
    .with_approximate_maxlen(true);  // More efficient for large streams

let consumer = RedisConsumer::<Event>::new(config);
```

### Message Acknowledgment

Configure automatic message acknowledgment:

```rust
use streamweave_redis::producers::{RedisProducer, RedisConsumerConfig};

let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("events")
    .with_group("my-group")
    .with_consumer("consumer-1")
    .with_auto_ack(true);  // Automatically acknowledge messages

let producer = RedisProducer::new(config);
```

### Error Handling

Configure error handling strategies:

```rust
use streamweave_redis::producers::{RedisProducer, RedisConsumerConfig};
use streamweave_error::ErrorStrategy;

let config = RedisConsumerConfig::default()
    .with_connection_url("redis://localhost:6379")
    .with_stream("events");

let producer = RedisProducer::new(config)
    .with_error_strategy(ErrorStrategy::Retry(5));  // Retry up to 5 times
```

## 🏗️ Architecture

Redis Streams integration flow:

```
┌──────────┐
│  Redis   │───> RedisProducer ───> Stream<RedisMessage> ───> Transformer ───> Stream<T> ───> RedisConsumer ───> Redis
│ Streams  │                                                                                                                      │ Streams  │
└──────────┘                                                                                                                      └──────────┘
```

**Redis Streams Flow:**
1. RedisProducer uses XREAD/XREADGROUP to consume messages
2. RedisMessage items flow through transformers
3. RedisConsumer serializes and uses XADD to send items to stream
4. Consumer groups manage message distribution
5. Message acknowledgment tracks processing status

## 🔧 Configuration

### Consumer Configuration (RedisConsumerConfig)

- **connection_url**: Redis connection URL (e.g., "redis://localhost:6379")
- **stream**: Stream name to consume from
- **group**: Consumer group name (optional, enables consumer groups)
- **consumer**: Consumer name (required if using consumer groups)
- **start_id**: Starting ID ("0" for beginning, "$" for new messages, ">" for consumer groups)
- **block_ms**: Block time in milliseconds (0 for non-blocking)
- **count**: Number of messages to read per call
- **auto_ack**: Whether to automatically acknowledge messages

### Producer Configuration (RedisProducerConfig)

- **connection_url**: Redis connection URL
- **stream**: Stream name to produce to
- **maxlen**: Maximum length of stream (None for no limit)
- **approximate_maxlen**: Use approximate maxlen (more efficient)

## 🔍 Error Handling

Redis errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = RedisProducer::new(config)
    .with_error_strategy(ErrorStrategy::Skip);  // Skip errors and continue

let consumer = RedisConsumer::<Event>::new(consumer_config)
    .with_error_strategy(ErrorStrategy::Retry(3));  // Retry up to 3 times
```

## ⚡ Performance Considerations

- **Stream Length**: Use maxlen to prevent unbounded growth
- **Approximate Maxlen**: Use approximate_maxlen for better performance
- **Blocking Reads**: Use block_ms for efficient polling
- **Batch Reads**: Use count to read multiple messages at once
- **Consumer Groups**: Use consumer groups for parallel processing

## 📝 Examples

For more examples, see:
- [Redis Streams Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/redis_streams_integration)
- [Redis-Specific Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-redis` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `redis` - Redis client library
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support
- `async-stream` - Stream utilities

## 🎯 Use Cases

Redis Streams integration is used for:

1. **Event Streaming**: Stream events from Redis Streams
2. **Message Queues**: Use Redis Streams as message queues
3. **Real-Time Processing**: Process Redis Streams messages in real-time
4. **Consumer Groups**: Distribute processing across consumers
5. **Event Sourcing**: Implement event sourcing patterns

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-redis)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/redis)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-error](../error/README.md) - Error handling
- [streamweave-message](../message/README.md) - Message envelopes
- [streamweave-offset](../offset/README.md) - Offset management

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

