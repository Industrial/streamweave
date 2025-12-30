# streamweave-stateful

[![Crates.io](https://img.shields.io/crates/v/streamweave-stateful.svg)](https://crates.io/crates/streamweave-stateful)
[![Documentation](https://docs.rs/streamweave-stateful/badge.svg)](https://docs.rs/streamweave-stateful)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Stateful transformer support for StreamWeave**  
*Transformers with persistent state for aggregations, sessions, and stateful processing.*

The `streamweave-stateful` package provides stateful transformer support for StreamWeave. It enables transformers to maintain persistent state across stream items, supporting use cases like running aggregations, session management, pattern detection, and stateful windowing operations.

## ✨ Key Features

- **StatefulTransformer Trait**: Extends Transformer with state management
- **StateStore Trait**: Abstract state storage interface
- **InMemoryStateStore**: Thread-safe in-memory state storage
- **State Persistence**: Serialization and checkpointing support
- **Thread-Safe**: All operations are thread-safe
- **State Operations**: Get, set, update, reset state

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-stateful = "0.6.0"
```

## 🚀 Quick Start

### Running Sum Transformer

```rust
use streamweave_stateful::{StatefulTransformer, InMemoryStateStore, StateStoreExt};
use streamweave::{Transformer, Input, Output};
use futures::StreamExt;
use std::pin::Pin;

struct RunningSumTransformer {
    state: InMemoryStateStore<i64>,
}

impl Input for RunningSumTransformer {
    type Input = i32;
    type InputStream = Pin<Box<dyn Stream<Item = i32> + Send>>;
}

impl Output for RunningSumTransformer {
    type Output = i64;
    type OutputStream = Pin<Box<dyn Stream<Item = i64> + Send>>;
}

impl Transformer for RunningSumTransformer {
    type InputPorts = (i32,);
    type OutputPorts = (i64,);
    
    fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
        Box::pin(input.map(move |x| {
            let sum = self.state.update(|current| {
                current.unwrap_or(0) + x as i64
            }).unwrap();
            sum
        }))
    }
    
    // ... config methods
}

impl StatefulTransformer for RunningSumTransformer {
    type State = i64;
    type Store = InMemoryStateStore<i64>;
    
    fn state_store(&self) -> &Self::Store {
        &self.state
    }
    
    fn state_store_mut(&mut self) -> &mut Self::Store {
        &mut self.state
    }
}
```

## 📖 API Overview

### StatefulTransformer Trait

The `StatefulTransformer` trait extends `Transformer` with state management:

```rust
pub trait StatefulTransformer: Transformer {
    type State: Clone + Send + Sync;
    type Store: StateStore<Self::State>;
    
    fn state_store(&self) -> &Self::Store;
    fn state_store_mut(&mut self) -> &mut Self::Store;
}
```

### StateStore Trait

The `StateStore` trait defines the interface for state storage:

```rust
pub trait StateStore<S> {
    fn get(&self) -> StateResult<Option<S>>;
    fn set(&self, state: S) -> StateResult<()>;
    fn update_with(&self, f: Box<dyn FnOnce(Option<S>) -> S + Send>) -> StateResult<S>;
    fn reset(&self) -> StateResult<()>;
    fn is_initialized(&self) -> bool;
    fn initial_state(&self) -> Option<S>;
}
```

### InMemoryStateStore

Thread-safe in-memory state storage:

```rust
pub struct InMemoryStateStore<S> {
    state: Arc<RwLock<Option<S>>>,
    initial: Option<S>,
}
```

## 📚 Usage Examples

### Running Aggregation

Calculate running sum, average, or count:

```rust
use streamweave_stateful::{InMemoryStateStore, StateStoreExt};

struct SumTransformer {
    state: InMemoryStateStore<i64>,
}

impl StatefulTransformer for SumTransformer {
    type State = i64;
    type Store = InMemoryStateStore<i64>;
    
    fn state_store(&self) -> &Self::Store { &self.state }
    fn state_store_mut(&mut self) -> &mut Self::Store { &mut self.state }
}

// In transform method
let sum = self.state.update(|current| {
    current.unwrap_or(0) + item
})?;
```

### State Persistence

Serialize and restore state:

```rust
use streamweave_stateful::{InMemoryStateStore, StateCheckpoint};

let store: InMemoryStateStore<i64> = InMemoryStateStore::new(42);

// Serialize state to bytes
let checkpoint = store.serialize_state()?;

// Restore to a new store
let store2: InMemoryStateStore<i64> = InMemoryStateStore::empty();
store2.deserialize_and_set_state(&checkpoint)?;

assert_eq!(store2.get()?, Some(42));
```

### State Reset

Reset state to initial value:

```rust
let store = InMemoryStateStore::new(0);
store.set(100)?;
store.reset()?;
assert_eq!(store.get()?, Some(0));
```

### State Initialization

Create stores with or without initial state:

```rust
// With initial state
let store = InMemoryStateStore::new(0);

// Without initial state
let store = InMemoryStateStore::<i64>::empty();

// With optional initial state
let store = InMemoryStateStore::with_optional_initial(Some(42));
```

### Thread-Safe State Access

All state operations are thread-safe:

```rust
use std::sync::Arc;
use tokio::task;

let store = Arc::new(InMemoryStateStore::new(0));

// Access from multiple threads
let store1 = Arc::clone(&store);
let store2 = Arc::clone(&store);

task::spawn(async move {
    store1.update(|current| current.unwrap_or(0) + 1).unwrap();
});

task::spawn(async move {
    store2.update(|current| current.unwrap_or(0) + 2).unwrap();
});
```

## 🏗️ Architecture

Stateful transformers maintain state across items:

```text
┌─────────────┐
│   Input     │───item───>┌──────────────────┐
└─────────────┘           │ StatefulTransformer│
                          │                    │
                          │  ┌──────────────┐  │
                          │  │ State Store  │  │
                          │  └──────────────┘  │
                          │                    │
                          └──────────────────┘
                                 │
                                 ▼
                          ┌─────────────┐
                          │   Output    │
                          └─────────────┘
```

**State Flow:**
1. Item arrives at transformer
2. Transformer reads current state
3. Transformer updates state based on item
4. Transformer produces output
5. State persists for next item

## 🔧 Configuration

### State Store Types

**InMemoryStateStore:**
- Thread-safe in-memory storage
- Fast access
- Not persistent across restarts
- Best for single-process use

**Custom Stores:**
- Implement `StateStore` trait
- Can use databases, files, etc.
- Enable distributed state
- Best for production use

## 🔍 Error Handling

State operations return `StateResult<T>`:

```rust
pub enum StateError {
    NotInitialized,
    LockPoisoned,
    UpdateFailed(String),
    SerializationFailed(String),
    DeserializationFailed(String),
}
```

## ⚡ Performance Considerations

- **Thread-Safe**: All operations use `RwLock` for thread safety
- **Clone Efficiency**: State cloning is efficient for small states
- **Serialization Overhead**: Checkpointing has serialization overhead
- **Lock Contention**: High contention may impact performance

## 📝 Examples

For more examples, see:
- [Stateful Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/stateful_processing)
- [Stateful Aggregation](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-stateful` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support
- `serde_json` - JSON serialization
- `chrono` - Timestamp support

## 🎯 Use Cases

Stateful transformers are used for:

1. **Running Aggregations**: Sum, average, count, min, max
2. **Session Management**: Track sessions across items
3. **Pattern Detection**: Detect patterns across multiple items
4. **Stateful Windowing**: Window operations with state
5. **State Persistence**: Checkpoint and restore state

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-stateful)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/stateful)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-pipeline](../pipeline/README.md) - Pipeline API
- [streamweave-graph](../graph/README.md) - Graph API
- [streamweave-window](../window/README.md) - Windowing operations

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

