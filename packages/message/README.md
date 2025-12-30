# streamweave-message

[![Crates.io](https://img.shields.io/crates/v/streamweave-message.svg)](https://crates.io/crates/streamweave-message)
[![Documentation](https://docs.rs/streamweave-message/badge.svg)](https://docs.rs/streamweave-message)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Message envelope types for StreamWeave**  
*Unique identifiers and metadata for exactly-once processing and message tracking.*

The `streamweave-message` package provides message envelope types that wrap stream items with unique identifiers and metadata. This enables features like message deduplication, offset tracking, exactly-once processing guarantees, and message flow tracking in pipelines and graphs.

## ✨ Key Features

- **Message Envelope**: Wrap payloads with unique IDs and metadata
- **MessageId Types**: UUID, Sequence, Custom, and Content-Hash identifiers
- **MessageMetadata**: Rich metadata (timestamp, source, partition, offset, key, headers)
- **ID Generators**: UUID, Sequence, and Content-Hash generators
- **Message Operations**: Map, transform, and unwrap messages
- **Exactly-Once Processing**: Enable deduplication and idempotency

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-message = "0.3.0"
```

## 🚀 Quick Start

### Creating Messages

```rust
use streamweave_message::{Message, MessageId, MessageMetadata};

// Create a simple message with UUID
let msg = Message::new(42, MessageId::new_uuid());

// Create a message with sequence ID
let msg = Message::new("hello", MessageId::new_sequence(1));

// Create a message with metadata
let metadata = MessageMetadata::with_timestamp_now()
    .source("my-source")
    .partition(0)
    .offset(100);

let msg = Message::with_metadata(
    "payload",
    MessageId::new_uuid(),
    metadata,
);
```

### Using ID Generators

```rust
use streamweave_message::{UuidGenerator, SequenceGenerator, IdGenerator};

// UUID generator (globally unique)
let uuid_gen = UuidGenerator::new();
let id1 = uuid_gen.next_id();
let id2 = uuid_gen.next_id();

// Sequence generator (monotonically increasing)
let seq_gen = SequenceGenerator::new();
let seq1 = seq_gen.next_id();  // Sequence(0)
let seq2 = seq_gen.next_id();  // Sequence(1)

// Sequence generator starting at specific value
let seq_gen = SequenceGenerator::starting_at(1000);
let seq = seq_gen.next_id();  // Sequence(1000)
```

## 📖 API Overview

### Message Type

The `Message<T>` type wraps a payload with an ID and metadata:

```rust
pub struct Message<T> {
    id: MessageId,
    payload: T,
    metadata: MessageMetadata,
}
```

**Key Methods:**
- `new(payload, id)` - Create message with payload and ID
- `with_metadata(payload, id, metadata)` - Create message with full metadata
- `id()` - Get message ID
- `payload()` - Get payload reference
- `metadata()` - Get metadata reference
- `map(f)` - Transform payload while preserving ID and metadata
- `into_payload()` - Extract payload, discarding envelope
- `into_parts()` - Extract all components

### MessageId Enum

The `MessageId` enum supports multiple ID types:

```rust
pub enum MessageId {
    Uuid(u128),           // UUID-based (128-bit)
    Sequence(u64),        // Sequence-based (64-bit)
    Custom(String),       // Custom string identifier
    ContentHash(u64),     // Content-hash based
}
```

**ID Types:**
- **Uuid**: Globally unique, good for distributed systems
- **Sequence**: Monotonically increasing, good for ordered processing
- **Custom**: User-provided identifier (e.g., from source system)
- **ContentHash**: Derived from content, useful for idempotency

### MessageMetadata

The `MessageMetadata` struct provides rich metadata:

```rust
pub struct MessageMetadata {
    pub timestamp: Option<Duration>,      // When message was created
    pub source: Option<String>,            // Source (topic, file, etc.)
    pub partition: Option<u32>,            // Partition/shard information
    pub offset: Option<u64>,                // Offset within partition
    pub key: Option<String>,                // Routing/grouping key
    pub headers: Vec<(String, String)>,    // Additional headers
}
```

### ID Generators

Multiple ID generator implementations:

**UuidGenerator:**
- Generates UUIDv4-style identifiers
- Globally unique
- Thread-safe

**SequenceGenerator:**
- Generates monotonically increasing sequence numbers
- Thread-safe using atomic operations
- Supports starting at specific value
- Can be reset

**ContentHashGenerator:**
- Generates IDs based on message content
- Useful for content-based idempotency
- Same content = same ID

## 📚 Usage Examples

### Creating Messages with Different ID Types

```rust
use streamweave_message::{Message, MessageId};

// UUID-based message
let msg = Message::new(42, MessageId::new_uuid());

// Sequence-based message
let msg = Message::new("data", MessageId::new_sequence(1));

// Custom ID message
let msg = Message::new(100, MessageId::new_custom("event-123"));

// Content-hash based message
let content = b"my content";
let msg = Message::new(content, MessageId::from_content(content));
```

### Working with Metadata

```rust
use streamweave_message::{Message, MessageId, MessageMetadata};

// Create metadata with builder pattern
let metadata = MessageMetadata::with_timestamp_now()
    .source("kafka-topic")
    .partition(3)
    .offset(1000)
    .key("user-123")
    .header("content-type", "application/json")
    .header("correlation-id", "req-456");

let msg = Message::with_metadata(
    "payload data",
    MessageId::new_uuid(),
    metadata,
);

// Access metadata
assert_eq!(msg.metadata().source, Some("kafka-topic".to_string()));
assert_eq!(msg.metadata().partition, Some(3));
assert_eq!(msg.metadata().get_header("content-type"), Some("application/json"));
```

### Transforming Messages

```rust
use streamweave_message::Message;

let msg = Message::new(42, MessageId::new_sequence(1));

// Map payload while preserving ID and metadata
let doubled = msg.map(|x| x * 2);
assert_eq!(*doubled.payload(), 84);
assert_eq!(*doubled.id(), MessageId::new_sequence(1));

// Map with access to message ID
let with_id = msg.map_with_id(|id, payload| {
    format!("{}:{}", id, payload)
});

// Replace payload
let new_msg = msg.with_payload("new payload");
```

### Using ID Generators

```rust
use streamweave_message::{UuidGenerator, SequenceGenerator, IdGenerator};

// UUID generator
let uuid_gen = UuidGenerator::new();
for _ in 0..10 {
    let id = uuid_gen.next_id();
    // Each ID is unique
}

// Sequence generator
let seq_gen = SequenceGenerator::new();
let id1 = seq_gen.next_id();  // Sequence(0)
let id2 = seq_gen.next_id();  // Sequence(1)

// Sequence generator with starting value
let seq_gen = SequenceGenerator::starting_at(100);
let id = seq_gen.next_id();  // Sequence(100)

// Reset sequence
seq_gen.reset();
let id = seq_gen.next_id();  // Sequence(0)

// Get current value without incrementing
let current = seq_gen.current();
```

### Message Flow in Pipelines

```rust
use streamweave_message::{Message, MessageId, MessageMetadata};
use streamweave::Transformer;

// Wrap items in messages
let messages: Vec<Message<i32>> = vec![1, 2, 3]
    .into_iter()
    .enumerate()
    .map(|(i, x)| {
        Message::with_metadata(
            x,
            MessageId::new_sequence(i as u64),
            MessageMetadata::with_timestamp_now()
                .source("input")
        )
    })
    .collect();

// Process messages (ID and metadata preserved)
let processed: Vec<Message<i32>> = messages
    .into_iter()
    .map(|msg| msg.map(|x| x * 2))
    .collect();

// Unwrap payloads when needed
let payloads: Vec<i32> = processed
    .into_iter()
    .map(|msg| msg.into_payload())
    .collect();
```

### Message Deduplication

```rust
use streamweave_message::{Message, MessageId};
use std::collections::HashSet;

// Track seen message IDs
let mut seen = HashSet::new();

let messages = vec![
    Message::new(1, MessageId::new_sequence(1)),
    Message::new(2, MessageId::new_sequence(2)),
    Message::new(1, MessageId::new_sequence(1)), // Duplicate
];

for msg in messages {
    if seen.insert(msg.id().clone()) {
        // Process unique message
        println!("Processing: {:?}", msg.payload());
    } else {
        // Skip duplicate
        println!("Skipping duplicate: {:?}", msg.id());
    }
}
```

### Message Routing by Key

```rust
use streamweave_message::{Message, MessageId, MessageMetadata};

let messages = vec![
    Message::with_metadata(
        "data1",
        MessageId::new_uuid(),
        MessageMetadata::new().key("user-1"),
    ),
    Message::with_metadata(
        "data2",
        MessageId::new_uuid(),
        MessageMetadata::new().key("user-2"),
    ),
    Message::with_metadata(
        "data3",
        MessageId::new_uuid(),
        MessageMetadata::new().key("user-1"),
    ),
];

// Route messages by key
let mut user1_messages = vec![];
let mut user2_messages = vec![];

for msg in messages {
    match msg.metadata().key.as_deref() {
        Some("user-1") => user1_messages.push(msg),
        Some("user-2") => user2_messages.push(msg),
        _ => {}
    }
}
```

## 🏗️ Architecture

Messages flow through pipelines and graphs with their envelope intact:

```text
┌─────────────┐
│  Producer   │───produces───> Message<T>
└─────────────┘
       │
       │ Message flows through
       ▼
┌─────────────┐
│ Transformer │───transforms───> Message<U> (ID preserved)
└─────────────┘
       │
       │ Message flows through
       ▼
┌─────────────┐
│  Consumer   │───consumes───> (can extract payload or keep envelope)
└─────────────┘
```

**Message Envelope Structure:**
```text
Message<T>
├── MessageId (unique identifier)
├── Payload<T> (actual data)
└── MessageMetadata
    ├── timestamp
    ├── source
    ├── partition
    ├── offset
    ├── key
    └── headers
```

## 🔗 Dependencies

`streamweave-message` depends on:

- `serde` - Serialization support
- `serde_json` - JSON serialization
- `chrono` - Timestamp support
- `streamweave` (optional) - Integration with core traits

## 🎯 Use Cases

Message envelopes are used for:

1. **Exactly-Once Processing**: Unique IDs enable deduplication
2. **Offset Tracking**: Track position in source streams
3. **Message Routing**: Route by key or partition
4. **Idempotency**: Content-hash IDs for content-based deduplication
5. **Message Correlation**: Track messages through complex pipelines
6. **Audit Trails**: Metadata provides full message history

## 🔍 Error Handling

Messages work seamlessly with the error handling system:

```rust
use streamweave_message::Message;
use streamweave_error::StreamError;

// Error context can include the message
let error_context = ErrorContext {
    timestamp: chrono::Utc::now(),
    item: Some(msg.clone()),  // Include message in error context
    component_name: "processor".to_string(),
    component_type: "Transformer".to_string(),
};
```

## ⚡ Performance Considerations

- **Zero-Copy**: Message operations are designed for efficiency
- **Clone Efficiency**: Messages clone efficiently when needed
- **Thread-Safe**: ID generators are thread-safe
- **Minimal Overhead**: Envelope adds minimal overhead to payloads

## 📝 Examples

For more examples, see:
- [Exactly-Once Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/exactly_once)
- [Message Deduplication](https://github.com/Industrial/streamweave/tree/main/examples)

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-message)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/message)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-offset](../offset/README.md) - Offset management
- [streamweave-transaction](../transaction/README.md) - Transaction support

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

