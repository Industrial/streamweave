# streamweave-transaction

[![Crates.io](https://img.shields.io/crates/v/streamweave-transaction.svg)](https://crates.io/crates/streamweave-transaction)
[![Documentation](https://docs.rs/streamweave-transaction/badge.svg)](https://docs.rs/streamweave-transaction)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Transaction management for StreamWeave**  
*All-or-nothing semantics for batch operations with offset buffering and savepoints.*

The `streamweave-transaction` package provides transactional processing capabilities for StreamWeave pipelines. It ensures all-or-nothing semantics for batches of operations, buffers offset commits within transactions, and supports savepoints for partial rollback.

## ✨ Key Features

- **Transaction Lifecycle**: Begin, commit, and rollback transactions
- **Offset Buffering**: Buffer offset commits within transactions
- **Timeout Support**: Configurable transaction timeouts with auto-rollback
- **Savepoints**: Nested transaction support with partial rollback
- **Transactional Context**: RAII-style transaction management
- **Metadata Support**: Custom metadata on transactions

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-transaction = "0.6.0"
```

## 🚀 Quick Start

### Basic Transaction

```rust
use streamweave_transaction::{TransactionManager, TransactionConfig};
use streamweave_offset::{InMemoryOffsetStore, Offset};
use std::time::Duration;

let offset_store = Box::new(InMemoryOffsetStore::new());
let config = TransactionConfig::default()
    .with_timeout(Duration::from_secs(30));
let manager = TransactionManager::new(offset_store, config);

// Begin a transaction
let tx_id = manager.begin().await?;

// Buffer some offset commits
manager.buffer_offset(&tx_id, "source1", Offset::Sequence(100)).await?;
manager.buffer_offset(&tx_id, "source2", Offset::Sequence(200)).await?;

// Commit the transaction (flushes all buffered offsets)
manager.commit(&tx_id).await?;
```

### Transactional Context

```rust
use streamweave_transaction::TransactionalContext;
use streamweave_offset::Offset;

let ctx = TransactionalContext::new(&manager).await?;

// Buffer offsets
ctx.buffer_offset("source1", Offset::Sequence(100)).await?;
ctx.buffer_offset("source2", Offset::Sequence(200)).await?;

// Commit (or rollback on error)
ctx.commit().await?;
```

## 📖 API Overview

### TransactionManager

The `TransactionManager` manages transaction lifecycle:

```rust
pub struct TransactionManager {
    offset_store: Arc<RwLock<Box<dyn OffsetStore>>>,
    config: TransactionConfig,
}
```

**Key Methods:**
- `begin()` - Begin a new transaction
- `begin_with_timeout(timeout)` - Begin with custom timeout
- `commit(id)` - Commit transaction, flushing buffered offsets
- `rollback(id)` - Rollback transaction, discarding buffered operations
- `buffer_offset(id, source, offset)` - Buffer an offset commit
- `create_savepoint(id, name)` - Create a savepoint
- `rollback_to_savepoint(id, name)` - Rollback to a savepoint

### Transaction States

```rust
pub enum TransactionState {
    Active,        // Transaction is active
    Committed,      // Transaction committed successfully
    RolledBack,     // Transaction rolled back
    TimedOut,       // Transaction timed out
}
```

### TransactionConfig

Configuration for transaction behavior:

```rust
pub struct TransactionConfig {
    pub timeout: Duration,                    // Default timeout
    pub max_nesting_level: usize,             // Max savepoint nesting
    pub auto_rollback_on_timeout: bool,       // Auto-rollback on timeout
}
```

### Savepoints

Savepoints enable partial rollback within transactions:

```rust
pub struct Savepoint {
    pub name: String,
    pub buffer_index: usize,
    pub created_at: Instant,
}
```

### TransactionalContext

RAII-style transaction management:

```rust
pub struct TransactionalContext<'a> {
    manager: &'a TransactionManager,
    id: TransactionId,
}
```

## 📚 Usage Examples

### Basic Transaction Flow

```rust
use streamweave_transaction::{TransactionManager, TransactionConfig};
use streamweave_offset::{InMemoryOffsetStore, Offset};

let store = Box::new(InMemoryOffsetStore::new());
let manager = TransactionManager::with_default_config(store);

// Begin transaction
let tx_id = manager.begin().await?;

// Buffer multiple offset commits
manager.buffer_offset(&tx_id, "topic-1", Offset::Sequence(100)).await?;
manager.buffer_offset(&tx_id, "topic-2", Offset::Sequence(200)).await?;
manager.buffer_offset(&tx_id, "topic-3", Offset::Sequence(300)).await?;

// Commit (all offsets are flushed atomically)
manager.commit(&tx_id).await?;
```

### Transaction Rollback

```rust
let tx_id = manager.begin().await?;

manager.buffer_offset(&tx_id, "source1", Offset::Sequence(100)).await?;
manager.buffer_offset(&tx_id, "source2", Offset::Sequence(200)).await?;

// On error, rollback (all buffered offsets are discarded)
if processing_failed {
    manager.rollback(&tx_id).await?;
} else {
    manager.commit(&tx_id).await?;
}
```

### Savepoints

Create savepoints for partial rollback:

```rust
let tx_id = manager.begin().await?;

// Process first batch
manager.buffer_offset(&tx_id, "source1", Offset::Sequence(100)).await?;
manager.buffer_offset(&tx_id, "source2", Offset::Sequence(200)).await?;

// Create savepoint
manager.create_savepoint(&tx_id, "batch1").await?;

// Process second batch
manager.buffer_offset(&tx_id, "source3", Offset::Sequence(300)).await?;
manager.buffer_offset(&tx_id, "source4", Offset::Sequence(400)).await?;

// If second batch fails, rollback to savepoint
if batch2_failed {
    manager.rollback_to_savepoint(&tx_id, "batch1").await?;
    // Only batch1 offsets remain buffered
}

// Commit (only successful batches)
manager.commit(&tx_id).await?;
```

### Transaction Timeouts

Configure timeouts for long-running transactions:

```rust
use std::time::Duration;

let config = TransactionConfig::default()
    .with_timeout(Duration::from_secs(30))
    .with_auto_rollback_on_timeout(true);

let manager = TransactionManager::new(store, config);

let tx_id = manager.begin().await?;

// Transaction will auto-rollback if it exceeds 30 seconds
// Check timeout status
let state = manager.get_state(&tx_id).await?;
if state == TransactionState::TimedOut {
    // Handle timeout
}
```

### Transactional Context

Use RAII-style transaction management:

```rust
use streamweave_transaction::TransactionalContext;

async fn process_batch(manager: &TransactionManager) -> Result<(), TransactionError> {
    let ctx = TransactionalContext::new(manager).await?;
    
    // Buffer offsets
    ctx.buffer_offset("source1", Offset::Sequence(100)).await?;
    ctx.buffer_offset("source2", Offset::Sequence(200)).await?;
    
    // Create savepoint
    ctx.savepoint("checkpoint").await?;
    
    // More operations
    ctx.buffer_offset("source3", Offset::Sequence(300)).await?;
    
    // On error, rollback to savepoint
    if error_occurred {
        ctx.rollback_to("checkpoint").await?;
    }
    
    // Commit (or rollback)
    ctx.commit().await?;
    
    Ok(())
}
```

### Custom Timeout per Transaction

Set custom timeout for specific transactions:

```rust
let tx_id = manager.begin_with_timeout(Duration::from_secs(60)).await?;

// This transaction has 60 second timeout
manager.buffer_offset(&tx_id, "source1", Offset::Sequence(100)).await?;
manager.commit(&tx_id).await?;
```

### Transaction Metadata

Attach custom metadata to transactions:

```rust
let tx_id = manager.begin().await?;

manager.set_metadata(&tx_id, "user_id", "123").await?;
manager.set_metadata(&tx_id, "operation", "batch_process").await?;

// Retrieve metadata
let user_id = manager.get_metadata(&tx_id, "user_id").await?;
```

### Transaction Monitoring

Monitor active transactions:

```rust
// List all active transactions
let active = manager.list_active().await;
println!("Active transactions: {:?}", active);

// Get transaction info
let info = manager.get_transaction(&tx_id).await?;
println!("Transaction {}: {:?}", info.id, info.state);
println!("Buffered offsets: {}", info.buffered_offset_count);
println!("Elapsed time: {:?}", info.elapsed);
println!("Remaining time: {:?}", info.remaining_time);
```

### Timeout Checking

Periodically check for timed-out transactions:

```rust
// Check for timed-out transactions
let timed_out = manager.check_timeouts().await;
for tx_id in timed_out {
    println!("Transaction {} timed out", tx_id);
    // Handle timeout
}

// Cleanup old completed transactions
let cleaned = manager.cleanup(Duration::from_secs(3600)).await;
println!("Cleaned up {} old transactions", cleaned);
```

### Nested Savepoints

Create multiple savepoints for complex rollback scenarios:

```rust
let tx_id = manager.begin().await?;

// First checkpoint
manager.buffer_offset(&tx_id, "s1", Offset::Sequence(1)).await?;
manager.create_savepoint(&tx_id, "checkpoint1").await?;

// Second checkpoint
manager.buffer_offset(&tx_id, "s2", Offset::Sequence(2)).await?;
manager.create_savepoint(&tx_id, "checkpoint2").await?;

// Third checkpoint
manager.buffer_offset(&tx_id, "s3", Offset::Sequence(3)).await?;
manager.create_savepoint(&tx_id, "checkpoint3").await?;

// Rollback to any checkpoint
manager.rollback_to_savepoint(&tx_id, "checkpoint1").await?;
// Only s1 remains buffered
```

## 🏗️ Architecture

Transactions buffer offset commits and flush them atomically:

```text
┌──────────────────┐
│ Transaction Begin│
└──────────────────┘
         │
         ▼
┌──────────────────┐
│ Buffer Offsets   │───> In-Memory Buffer
└──────────────────┘
         │
         ▼
┌──────────────────┐
│ Create Savepoint │───> Savepoint Stack
└──────────────────┘
         │
         ▼
┌──────────────────┐
│ Commit/Rollback  │───> Flush to OffsetStore (or discard)
└──────────────────┘
```

**Transaction Flow:**
1. Begin transaction
2. Buffer offset commits (not yet persisted)
3. Optionally create savepoints
4. Commit (flushes all buffered offsets) or Rollback (discards all)

## 🔧 Configuration

### TransactionConfig Options

**Timeout:**
- Default: 60 seconds
- Configurable per transaction or globally
- Auto-rollback on timeout (configurable)

**Nesting Level:**
- Default: 10 savepoints
- Prevents excessive nesting
- Configurable limit

**Auto-Rollback:**
- Default: true
- Automatically clears buffered operations on timeout
- Can be disabled for manual handling

## 🔍 Error Handling

Transaction operations return `TransactionResult<T>`:

```rust
pub enum TransactionError {
    NotFound(TransactionId),                    // Transaction not found
    NotActive(TransactionId, TransactionState),  // Transaction not active
    Timeout(TransactionId),                      // Transaction timed out
    OffsetError(OffsetError),                    // Offset operation failed
    LockError(String),                           // Lock acquisition failed
    InvalidOperation(String),                    // Invalid operation
    SavepointNotFound(String),                   // Savepoint not found
    NestingLimitExceeded(usize),                 // Too many savepoints
}
```

## ⚡ Performance Considerations

- **In-Memory Buffering**: Offsets are buffered in memory until commit
- **Atomic Commits**: All offsets in a transaction are committed atomically
- **Timeout Overhead**: Periodic timeout checking has minimal overhead
- **Savepoint Overhead**: Savepoints add minimal overhead (just buffer indices)

## 📝 Examples

For more examples, see:
- [Exactly-Once Processing Example](https://github.com/Industrial/streamweave/tree/main/examples/exactly_once)
- [Transaction Management](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-transaction` depends on:

- `streamweave` - Core traits
- `streamweave-offset` - Offset management
- `streamweave-message` (optional) - Message envelope support
- `tokio` - Async runtime
- `chrono` - Timestamp support
- `futures` - Future utilities

## 🎯 Use Cases

Transaction management is used for:

1. **Exactly-Once Processing**: Atomic offset commits ensure no duplicates
2. **Batch Processing**: Process multiple items atomically
3. **Error Recovery**: Rollback on errors to maintain consistency
4. **Partial Rollback**: Use savepoints for complex error handling
5. **Long-Running Operations**: Timeout support for safety

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-transaction)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/transaction)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-offset](../offset/README.md) - Offset management
- [streamweave-message](../message/README.md) - Message envelopes

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

