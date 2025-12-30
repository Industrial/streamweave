# streamweave-kafka

[![Crates.io](https://img.shields.io/crates/v/streamweave-kafka.svg)](https://crates.io/crates/streamweave-kafka)
[![Documentation](https://docs.rs/streamweave-kafka/badge.svg)](https://docs.rs/streamweave-kafka)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Apache Kafka integration for StreamWeave**  
*Produce to and consume from Kafka topics with streaming processing.*

The `streamweave-kafka` package provides Kafka producers and consumers for StreamWeave. It enables reading from Kafka topics and writing to Kafka topics with consumer groups, offset management, batching, and comprehensive error handling.

## ✨ Key Features

- **KafkaProducer**: Consume messages from Kafka topics and stream them
- **KafkaConsumer**: Produce messages to Kafka topics from streams
- **Consumer Groups**: Support for Kafka consumer groups
- **Offset Management**: Automatic and manual offset management
- **Batching**: Batch message production for performance
- **Error Handling**: Comprehensive error handling with retry strategies
- **Compression**: Support for various compression types

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-kafka = { version = "0.3.0", features = ["kafka"] }
```

## 🚀 Quick Start

### Consume from Kafka

```rust
use streamweave_kafka::{KafkaProducer, KafkaConsumerConfig};
use streamweave_pipeline::PipelineBuilder;

let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_group_id("my-consumer-group")
    .with_topic("my-topic")
    .with_auto_offset_reset("earliest");

let pipeline = PipelineBuilder::new()
    .producer(KafkaProducer::new(config))
    .consumer(/* process messages */);

pipeline.run().await?;
```

### Produce to Kafka

```rust
use streamweave_kafka::{KafkaConsumer, KafkaProducerConfig};
use streamweave_pipeline::PipelineBuilder;
use serde::Serialize;

#[derive(Serialize)]
struct Event {
    id: u32,
    message: String,
}

let config = KafkaProducerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_topic("my-topic")
    .with_acks("all");

let pipeline = PipelineBuilder::new()
    .producer(/* produce events */)
    .consumer(KafkaConsumer::<Event>::new(config));

pipeline.run().await?;
```

## 📖 API Overview

### KafkaProducer

Consumes messages from Kafka topics and streams them:

```rust
pub struct KafkaProducer {
    pub config: ProducerConfig<KafkaMessage>,
    pub kafka_config: KafkaConsumerConfig,
}
```

**Key Methods:**
- `new(config)` - Create producer with Kafka configuration
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `produce()` - Generate stream from Kafka messages

### KafkaConsumer

Produces messages to Kafka topics from streams:

```rust
pub struct KafkaConsumer<T> {
    pub config: ConsumerConfig<T>,
    pub kafka_config: KafkaProducerConfig,
}
```

**Key Methods:**
- `new(config)` - Create consumer with Kafka configuration
- `with_error_strategy(strategy)` - Set error handling strategy
- `with_name(name)` - Set component name
- `consume(stream)` - Send stream items to Kafka topic

### KafkaMessage

Represents a message received from Kafka:

```rust
pub struct KafkaMessage {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
    pub key: Option<Vec<u8>>,
    pub payload: Vec<u8>,
    pub timestamp: Option<i64>,
    pub headers: HashMap<String, Vec<u8>>,
}
```

## 📚 Usage Examples

### Consumer Group Setup

Configure consumer groups for distributed processing:

```rust
use streamweave_kafka::{KafkaProducer, KafkaConsumerConfig};

let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("kafka1:9092,kafka2:9092")
    .with_group_id("my-consumer-group")
    .with_topic("events")
    .with_auto_offset_reset("earliest")
    .with_enable_auto_commit(true)
    .with_auto_commit_interval_ms(5000)
    .with_session_timeout_ms(30000)
    .with_max_poll_interval_ms(300000);

let producer = KafkaProducer::new(config);
```

### Multiple Topics

Consume from multiple topics:

```rust
use streamweave_kafka::{KafkaProducer, KafkaConsumerConfig};

let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_group_id("multi-topic-consumer")
    .with_topics(vec!["topic1".to_string(), "topic2".to_string()])
    .with_auto_offset_reset("latest");

let producer = KafkaProducer::new(config);
```

### Offset Management

Configure offset management:

```rust
use streamweave_kafka::{KafkaProducer, KafkaConsumerConfig};

let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_group_id("offset-managed-consumer")
    .with_topic("events")
    .with_auto_offset_reset("earliest")  // or "latest"
    .with_enable_auto_commit(false)  // Manual offset commits
    .with_auto_commit_interval_ms(1000);

let producer = KafkaProducer::new(config);
```

### Producer Configuration

Configure Kafka producer settings:

```rust
use streamweave_kafka::{KafkaConsumer, KafkaProducerConfig};

let config = KafkaProducerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_topic("events")
    .with_client_id("my-producer")
    .with_acks("all")  // Wait for all replicas
    .with_retries(3)
    .with_batch_size(16384)  // 16KB batches
    .with_linger_ms(10)  // Wait 10ms to batch
    .with_compression_type("gzip")
    .with_max_request_size(1048576);  // 1MB max

let consumer = KafkaConsumer::<Event>::new(config);
```

### Error Handling

Configure error handling strategies:

```rust
use streamweave_kafka::{KafkaProducer, KafkaConsumerConfig};
use streamweave_error::ErrorStrategy;

let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_topic("events");

let producer = KafkaProducer::new(config)
    .with_error_strategy(ErrorStrategy::Retry(5));  // Retry up to 5 times
```

### Custom Properties

Set custom Kafka properties:

```rust
use streamweave_kafka::{KafkaProducer, KafkaConsumerConfig};

let config = KafkaConsumerConfig::default()
    .with_bootstrap_servers("localhost:9092")
    .with_topic("events")
    .with_custom_property("fetch.min.bytes", "1024")
    .with_custom_property("max.partition.fetch.bytes", "1048576");

let producer = KafkaProducer::new(config);
```

## 🏗️ Architecture

Kafka integration flow:

```text
┌──────────┐
│  Kafka   │───> KafkaProducer ───> Stream<KafkaMessage> ───> Transformer ───> Stream<T> ───> KafkaConsumer ───> Kafka
└──────────┘                                                                                                                      └──────────┘
```

**Kafka Flow:**
1. KafkaProducer subscribes to topics and consumes messages
2. KafkaMessage items flow through transformers
3. KafkaConsumer serializes and sends items to Kafka topic
4. Consumer groups manage partition assignment
5. Offset management tracks consumption progress

## 🔧 Configuration

### Consumer Configuration (KafkaConsumerConfig)

- **bootstrap_servers**: Comma-separated list of Kafka brokers
- **group_id**: Consumer group identifier
- **topics**: List of topics to consume from
- **auto_offset_reset**: "earliest" or "latest"
- **enable_auto_commit**: Enable automatic offset commits
- **auto_commit_interval_ms**: Interval for auto commits
- **session_timeout_ms**: Session timeout for consumer group
- **max_poll_interval_ms**: Maximum time between polls
- **fetch_max_bytes**: Maximum bytes per fetch
- **fetch_wait_max_ms**: Maximum wait time for fetch
- **custom_properties**: Additional Kafka properties

### Producer Configuration (KafkaProducerConfig)

- **bootstrap_servers**: Comma-separated list of Kafka brokers
- **topic**: Topic to produce to
- **client_id**: Client identifier
- **acks**: "0", "1", or "all"
- **retries**: Maximum number of retries
- **batch_size**: Batch size in bytes
- **linger_ms**: Wait time before sending batch
- **max_request_size**: Maximum request size
- **compression_type**: "none", "gzip", "snappy", "lz4", "zstd"
- **custom_properties**: Additional Kafka properties

## 🔍 Error Handling

Kafka errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let producer = KafkaProducer::new(config)
    .with_error_strategy(ErrorStrategy::Skip);  // Skip errors and continue

let consumer = KafkaConsumer::<Event>::new(consumer_config)
    .with_error_strategy(ErrorStrategy::Retry(3));  // Retry up to 3 times
```

## ⚡ Performance Considerations

- **Batching**: Use batch_size and linger_ms for better throughput
- **Compression**: Enable compression for better network efficiency
- **Consumer Groups**: Use consumer groups for parallel processing
- **Fetch Settings**: Tune fetch_max_bytes and fetch_wait_max_ms
- **Acks**: Use "all" for durability, "1" for performance

## 📝 Examples

For more examples, see:
- [Kafka Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/kafka_integration)
- [Kafka-Specific Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-kafka` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-message` (optional) - Message envelope support
- `rdkafka` - Kafka client library
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support
- `async-stream` - Stream utilities

## 🎯 Use Cases

Kafka integration is used for:

1. **Event Streaming**: Stream events from Kafka topics
2. **Data Pipeline**: Build data pipelines with Kafka
3. **Real-Time Processing**: Process Kafka messages in real-time
4. **Event Sourcing**: Implement event sourcing patterns
5. **Microservices**: Integrate microservices via Kafka

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-kafka)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/kafka)
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

