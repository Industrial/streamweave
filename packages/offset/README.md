# streamweave-offset

[![Crates.io](https://img.shields.io/crates/v/streamweave-offset.svg)](https://crates.io/crates/streamweave-offset)
[![Documentation](https://docs.rs/streamweave-offset/badge.svg)](https://docs.rs/streamweave-offset)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Offset management for StreamWeave**  
*Track processing positions for exactly-once processing and resumable pipelines.*

The `streamweave-offset` package provides offset tracking and management for StreamWeave pipelines. It enables exactly-once processing guarantees by tracking the position of processed items in source streams, allowing pipelines to resume from where they left off after restarts.

## ✨ Key Features

- **Offset Types**: Sequence, Timestamp, Custom, Earliest, Latest
- **Offset Storage**: In-memory and file-based storage backends
- **Offset Tracker**: High-level API for offset management
- **Commit Strategies**: Auto, Periodic, and Manual commit modes
- **Reset Policies**: Configurable behavior when no offset is found
- **Persistence**: File-based persistence for recovery after restarts

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-offset = "0.3.0"
```

## 🚀 Quick Start

### Basic Offset Tracking

```rust
use streamweave_offset::{Offset, OffsetTracker, InMemoryOffsetStore, CommitStrategy};

// Create an in-memory offset store
let store = Box::new(InMemoryOffsetStore::new());

// Create an offset tracker with auto-commit
let tracker = OffsetTracker::new(store);

// Record processed offsets
tracker.record("my-source", Offset::Sequence(100))?;
tracker.record("my-source", Offset::Sequence(101))?;

// Get the current offset
let current = tracker.get_offset("my-source")?;
assert_eq!(current, Offset::Sequence(101));
```

### File-Based Persistence

```rust
use streamweave_offset::{Offset, OffsetTracker, FileOffsetStore};

// Create a file-based offset store
let store = Box::new(FileOffsetStore::new("offsets.json")?);

// Create tracker
let tracker = OffsetTracker::new(store);

// Record offsets (automatically persisted)
tracker.record("kafka-topic", Offset::Sequence(1000))?;

// After restart, offsets are automatically loaded
let tracker2 = OffsetTracker::new(Box::new(FileOffsetStore::new("offsets.json")?));
let offset = tracker2.get_offset("kafka-topic")?;
// offset is Sequence(1000)
```

## 📖 API Overview

### Offset Enum

The `Offset` enum represents different types of offsets:

```rust
pub enum Offset {
    Sequence(u64),              // Sequence number offset
    Timestamp(DateTime<Utc>),    // Timestamp-based offset
    Custom(String),              // Custom string offset
    Earliest,                    // Beginning of stream
    Latest,                      // End of stream (latest)
}
```

**Offset Types:**
- **Sequence**: Numeric sequence numbers (e.g., Kafka partition offsets)
- **Timestamp**: Time-based offsets (e.g., event timestamps)
- **Custom**: String-based offsets (e.g., file positions, custom IDs)
- **Earliest**: Start from the beginning of the stream
- **Latest**: Start from the latest available position

### OffsetStore Trait

The `OffsetStore` trait defines the interface for offset storage:

```rust
pub trait OffsetStore {
    fn get(&self, source: &str) -> OffsetResult<Option<Offset>>;
    fn commit(&self, source: &str, offset: Offset) -> OffsetResult<()>;
    fn get_all(&self) -> OffsetResult<HashMap<String, Offset>>;
    fn clear(&self, source: &str) -> OffsetResult<()>;
    fn clear_all(&self) -> OffsetResult<()>;
}
```

**Implementations:**
- `InMemoryOffsetStore` - In-memory storage (for testing)
- `FileOffsetStore` - File-based persistence (JSON)

### OffsetTracker

The `OffsetTracker` provides a high-level API for offset management:

```rust
pub struct OffsetTracker {
    store: Box<dyn OffsetStore>,
    strategy: CommitStrategy,
    reset_policy: OffsetResetPolicy,
}
```

**Key Methods:**
- `get_offset(source)` - Get current committed offset
- `record(source, offset)` - Record a processed offset
- `commit(source)` - Manually commit pending offset
- `commit_all()` - Commit all pending offsets
- `reset(source, offset)` - Reset offset to specific value
- `clear(source)` - Clear offset for a source

### CommitStrategy

The `CommitStrategy` enum defines when offsets are committed:

```rust
pub enum CommitStrategy {
    Auto,                    // Commit after each item
    Periodic(usize),         // Commit every N items
    Manual,                  // Only commit when explicitly requested
}
```

### OffsetResetPolicy

The `OffsetResetPolicy` enum defines behavior when no offset is found:

```rust
pub enum OffsetResetPolicy {
    Earliest,    // Start from beginning (default)
    Latest,      // Start from latest position
    None,        // Fail if no offset found
}
```

## 📚 Usage Examples

### Auto-Commit Strategy

Commit offsets immediately after each item:

```rust
use streamweave_offset::{Offset, OffsetTracker, InMemoryOffsetStore, CommitStrategy};

let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::new(store);

// Each record is immediately committed
tracker.record("source", Offset::Sequence(1))?;
tracker.record("source", Offset::Sequence(2))?;

// Offset is immediately available
let offset = tracker.get_offset("source")?;
assert_eq!(offset, Offset::Sequence(2));
```

### Periodic Commit Strategy

Commit offsets every N items:

```rust
use streamweave_offset::{Offset, OffsetTracker, InMemoryOffsetStore, CommitStrategy};

let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::with_strategy(
    store,
    CommitStrategy::Periodic(10)  // Commit every 10 items
);

// Record 9 items (not yet committed)
for i in 1..=9 {
    tracker.record("source", Offset::Sequence(i))?;
}
assert!(tracker.get_all_committed()?.is_empty());

// 10th item triggers commit
tracker.record("source", Offset::Sequence(10))?;
let offset = tracker.get_offset("source")?;
assert_eq!(offset, Offset::Sequence(10));
```

### Manual Commit Strategy

Commit offsets only when explicitly requested:

```rust
use streamweave_offset::{Offset, OffsetTracker, InMemoryOffsetStore, CommitStrategy};

let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::with_strategy(
    store,
    CommitStrategy::Manual
);

// Record offsets (not committed)
tracker.record("source", Offset::Sequence(100))?;
tracker.record("source", Offset::Sequence(200))?;

// Check pending offsets
let pending = tracker.get_all_pending()?;
assert_eq!(pending.get("source"), Some(&Offset::Sequence(200)));

// Manually commit
tracker.commit("source")?;

// Now committed
let offset = tracker.get_offset("source")?;
assert_eq!(offset, Offset::Sequence(200));
```

### Offset Reset Policies

Handle missing offsets with reset policies:

```rust
use streamweave_offset::{Offset, OffsetTracker, InMemoryOffsetStore, OffsetResetPolicy};

// Earliest policy (default) - start from beginning
let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::new(store)
    .with_reset_policy(OffsetResetPolicy::Earliest);

let offset = tracker.get_offset("new-source")?;
assert_eq!(offset, Offset::Earliest);

// Latest policy - start from latest
let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::new(store)
    .with_reset_policy(OffsetResetPolicy::Latest);

let offset = tracker.get_offset("new-source")?;
assert_eq!(offset, Offset::Latest);

// None policy - fail if no offset found
let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::new(store)
    .with_reset_policy(OffsetResetPolicy::None);

let result = tracker.get_offset("new-source");
assert!(result.is_err());
```

### File-Based Persistence

Persist offsets to disk for recovery:

```rust
use streamweave_offset::{Offset, OffsetTracker, FileOffsetStore};

// Create file-based store
let store = Box::new(FileOffsetStore::new("offsets.json")?);
let tracker = OffsetTracker::new(store);

// Record offsets (automatically persisted to disk)
tracker.record("topic-1", Offset::Sequence(100))?;
tracker.record("topic-2", Offset::Sequence(200))?;

// After restart, create new tracker with same file
let store2 = Box::new(FileOffsetStore::new("offsets.json")?);
let tracker2 = OffsetTracker::new(store2);

// Offsets are automatically loaded
let offset1 = tracker2.get_offset("topic-1")?;
let offset2 = tracker2.get_offset("topic-2")?;
assert_eq!(offset1, Offset::Sequence(100));
assert_eq!(offset2, Offset::Sequence(200));
```

### Offset Management in Consumers

Use offsets in consumer implementations:

```rust
use streamweave_offset::{Offset, OffsetTracker, FileOffsetStore};
use streamweave::Consumer;

// Load or create offset tracker
let store = Box::new(FileOffsetStore::new("consumer-offsets.json")?);
let tracker = OffsetTracker::new(store);

// Get starting offset
let start_offset = tracker.get_offset("kafka-topic")?;

// Create consumer starting from offset
let consumer = KafkaConsumer::new()
    .with_start_offset(start_offset);

// Process messages
let mut current_offset = start_offset;
for message in messages {
    // Process message
    process_message(&message)?;
    
    // Update offset
    current_offset = current_offset.increment().unwrap();
    tracker.record("kafka-topic", current_offset.clone())?;
}
```

### Offset Recovery Scenarios

Handle recovery after failures:

```rust
use streamweave_offset::{Offset, OffsetTracker, FileOffsetStore, OffsetResetPolicy};

// After restart, load offsets
let store = Box::new(FileOffsetStore::new("offsets.json")?);
let tracker = OffsetTracker::new(store)
    .with_reset_policy(OffsetResetPolicy::Earliest);

// Get offset for each source
let sources = vec!["topic-1", "topic-2", "topic-3"];
for source in sources {
    let offset = tracker.get_offset(source)?;
    
    match offset {
        Offset::Earliest => {
            // New source, start from beginning
            println!("Starting {} from beginning", source);
        }
        Offset::Sequence(n) => {
            // Resume from last committed offset
            println!("Resuming {} from offset {}", source, n);
        }
        _ => {}
    }
}
```

### Multiple Sources

Track offsets for multiple sources:

```rust
use streamweave_offset::{Offset, OffsetTracker, InMemoryOffsetStore};

let store = Box::new(InMemoryOffsetStore::new());
let tracker = OffsetTracker::new(store);

// Track offsets for multiple sources
tracker.record("kafka-topic-1", Offset::Sequence(100))?;
tracker.record("kafka-topic-2", Offset::Sequence(200))?;
tracker.record("file-source", Offset::Custom("line-5000".to_string()))?;

// Get all committed offsets
let all_offsets = tracker.get_all_committed()?;
assert_eq!(all_offsets.len(), 3);

// Commit all pending offsets
tracker.commit_all()?;
```

## 🏗️ Architecture

Offset tracking integrates with consumers for exactly-once processing:

```
┌─────────────┐
│  Consumer   │───processes item───> Offset
└─────────────┘                           │
                                          │
                                          ▼
┌─────────────┐                    ┌──────────────┐
│OffsetTracker│───records───>       │ OffsetStore  │
└─────────────┘                    └──────────────┘
         │                                  │
         │                                  ▼
         └───commits (based on strategy)───> Persistence
```

**Offset Flow:**
1. Consumer processes item at offset N
2. OffsetTracker records offset N
3. Based on CommitStrategy, offset is committed
4. OffsetStore persists offset
5. On restart, offsets are loaded and processing resumes

## 🔧 Configuration

### Commit Strategies

**Auto (Default):**
- Commits after each item
- Maximum safety, higher I/O
- Best for critical data

**Periodic:**
- Commits every N items
- Balance between safety and performance
- Best for high-throughput scenarios

**Manual:**
- Commits only when requested
- Maximum control
- Best for transactional scenarios

### Reset Policies

**Earliest (Default):**
- Starts from beginning if no offset found
- Safe default for new sources
- May reprocess data

**Latest:**
- Starts from latest position
- Skips old data
- Best for real-time processing

**None:**
- Fails if no offset found
- Explicit error handling
- Best for strict requirements

## 🔍 Error Handling

Offset operations return `OffsetResult<T>` which can be:

```rust
pub enum OffsetError {
    IoError(io::Error),              // File I/O errors
    SerializationError(String),       // JSON serialization errors
    SourceNotFound(String),           // Source not found
    LockError(String),                // Lock acquisition failed
    InvalidOffset(String),            // Invalid offset format
}
```

## ⚡ Performance Considerations

- **In-Memory Store**: Fast but not persistent
- **File Store**: Persistent with JSON serialization overhead
- **Auto Commit**: Higher I/O, maximum safety
- **Periodic Commit**: Reduced I/O, batch efficiency
- **Manual Commit**: Minimal I/O, maximum control

## 📝 Examples

For more examples, see:
- [Exactly-Once Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/exactly_once)
- [Kafka Integration](https://github.com/Industrial/streamweave/tree/main/examples/kafka_integration)

## 🔗 Dependencies

`streamweave-offset` depends on:

- `chrono` - Timestamp support
- `serde` - Serialization support
- `serde_json` - JSON serialization
- `streamweave` - Core traits

## 🎯 Use Cases

Offset management is used for:

1. **Exactly-Once Processing**: Track processed items to avoid duplicates
2. **Resumable Pipelines**: Resume from last processed position after restart
3. **Kafka Integration**: Track partition offsets for consumer groups
4. **File Processing**: Track line numbers or byte positions
5. **Recovery**: Recover from failures without data loss

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-offset)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/offset)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-message](../message/README.md) - Message envelopes
- [streamweave-transaction](../transaction/README.md) - Transaction support

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

