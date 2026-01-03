# Migration Guide: Universal Message Model

This guide explains how to migrate your code from the old raw-type model to the new universal `Message<T>` model in StreamWeave.

## Overview

**All data in StreamWeave now flows as `Message<T>`.** This change enables:
- End-to-end message traceability
- Metadata preservation through transformations
- Better error correlation
- Zero-copy sharing in fan-out scenarios

## Quick Migration Paths

### Option 1: Use Adapters (Easiest)

If you have existing code that works with raw types, use adapters to integrate with the message-based system:

```rust
use streamweave::adapters::{MessageWrapper, PayloadExtractor, PayloadExtractorConsumer};

// Wrap your existing producer
let raw_producer = MyRawProducer::new();
let producer = MessageWrapper::new(raw_producer);  // Now produces Message<T>

// Extract payloads in your transformer
let raw_transformer = MyRawTransformer::new();
let transformer = PayloadExtractor::new(raw_transformer);  // Works with raw types internally

// Extract payloads in your consumer
let raw_consumer = MyRawConsumer::new();
let consumer = PayloadExtractorConsumer::new(raw_consumer);  // Receives raw types
```

**When to use adapters:**
- Quick migration of existing code
- Simple transformations that don't need metadata
- Prototyping

### Option 2: Direct Message Usage (Recommended for New Code)

For new code or when you need message features, work directly with `Message<T>`:

```rust
use streamweave::message::{Message, MessageId, wrap_message};

// Create messages
let msg = wrap_message(42);  // Auto-generates UUID

// Access components
let payload = msg.payload();      // &i32
let id = msg.id();                // &MessageId
let metadata = msg.metadata();    // &MessageMetadata

// Transform while preserving ID and metadata
let doubled = msg.map(|x| x * 2);
```

**When to use messages directly:**
- Need message tracking through the pipeline
- Want to preserve or modify metadata
- Building advanced routing or correlation logic
- Error handling that needs message context

## Migration Steps by Component Type

### Migrating Producers

**Before (Raw Types):**
```rust
impl Producer for MyProducer {
    type Output = i32;  // Raw type
    
    fn produce(&mut self) -> Self::OutputStream {
        Box::pin(futures::stream::iter(self.items.clone()))
    }
}
```

**After (Messages):**
```rust
use streamweave::message::wrap_message;

impl Producer for MyProducer {
    type Output = Message<i32>;  // Message type
    
    fn produce(&mut self) -> Self::OutputStream {
        let items = self.items.clone();
        Box::pin(futures::stream::iter(
            items.into_iter().map(|item| wrap_message(item))
        ))
    }
}
```

**Or Use Adapter:**
```rust
use streamweave::adapters::MessageWrapper;

// Keep your raw producer as-is
struct MyRawProducer { /* ... */ }

// Wrap it
let producer = MessageWrapper::new(MyRawProducer::new());
```

### Migrating Transformers

**Before (Raw Types):**
```rust
impl Transformer for MyTransformer {
    type Input = i32;
    type Output = i64;
    
    async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
        Box::pin(stream.map(|x| x as i64))
    }
}
```

**After (Messages):**
```rust
use streamweave::message::Message;

impl Transformer for MyTransformer {
    type Input = Message<i32>;
    type Output = Message<i64>;
    
    async fn transform(&mut self, stream: Self::InputStream) -> Self::OutputStream {
        Box::pin(stream.map(|msg| {
            let payload = msg.payload().clone();
            let id = msg.id().clone();
            let metadata = msg.metadata().clone();
            Message::with_metadata(payload as i64, id, metadata)
        }))
    }
}
```

**Or Use Adapter:**
```rust
use streamweave::adapters::PayloadExtractor;

// Keep your raw transformer as-is
struct MyRawTransformer { /* ... */ }

// Wrap it
let transformer = PayloadExtractor::new(MyRawTransformer::new());
```

### Migrating Consumers

**Before (Raw Types):**
```rust
impl Consumer for MyConsumer {
    type Input = i32;
    
    async fn consume(&mut self, mut stream: Self::InputStream) {
        while let Some(item) = stream.next().await {
            self.items.push(item);
        }
    }
}
```

**After (Messages):**
```rust
use streamweave::message::Message;
use futures::StreamExt;

impl Consumer for MyConsumer {
    type Input = Message<i32>;
    
    async fn consume(&mut self, mut stream: Self::InputStream) {
        while let Some(msg) = stream.next().await {
            let payload = msg.payload();  // Access payload
            let id = msg.id();            // Access ID for tracking
            self.items.push(msg);
        }
    }
}
```

**Or Use Adapter:**
```rust
use streamweave::adapters::PayloadExtractorConsumer;

// Keep your raw consumer as-is
struct MyRawConsumer { /* ... */ }

// Wrap it
let consumer = PayloadExtractorConsumer::new(MyRawConsumer::new());
```

## Common Patterns

### Creating Messages

```rust
use streamweave::message::{Message, MessageId, wrap_message, MessageMetadata};

// Simple creation (auto-generates UUID)
let msg = wrap_message(42);

// With specific ID
let msg = Message::new(42, MessageId::new_uuid());

// With custom metadata
let metadata = MessageMetadata::default()
    .source("my_source")
    .header("key", "value");
let msg = Message::with_metadata(42, MessageId::new_uuid(), metadata);
```

### Accessing Message Components

```rust
let msg = wrap_message(42);

// Access payload (borrowed)
let payload = msg.payload();      // &i32

// Access ID
let id = msg.id();                // &MessageId

// Access metadata
let metadata = msg.metadata();     // &MessageMetadata

// Extract payload (consumes message)
let value = msg.into_payload();   // i32
```

### Transforming Messages

```rust
let msg = wrap_message(42);

// Transform payload while preserving ID and metadata
let doubled = msg.map(|x| x * 2);

// Manual transformation (preserve ID and metadata)
let payload = msg.payload().clone();
let id = msg.id().clone();
let metadata = msg.metadata().clone();
let new_msg = Message::with_metadata(payload * 2, id, metadata);
```

### Working with Message Streams

```rust
use streamweave::message::{MessageStreamExt, Message, MessageId};
use futures::StreamExt;

// Extract payloads
let payloads: Vec<i32> = message_stream
    .extract_payloads()
    .collect()
    .await;

// Extract IDs
let ids: Vec<MessageId> = message_stream
    .extract_ids()
    .collect()
    .await;

// Map payloads
let doubled: Vec<Message<i32>> = message_stream
    .map_payload(|x| x * 2)
    .collect()
    .await;
```

## Configuration Updates

All configuration types now work with `Message<T>`:

**Before:**
```rust
let config = ProducerConfig::<i32>::default();
```

**After:**
```rust
let config = ProducerConfig::<Message<i32>>::default();
```

## Error Handling Updates

Error contexts now include the full `Message<T>` that caused the error:

```rust
// In your error handler
fn handle_error(&self, error: &StreamError<Message<i32>>) -> ErrorAction {
    if let Some(msg) = &error.context.item {
        // Access message ID
        let id = msg.id();
        
        // Access message metadata
        let metadata = msg.metadata();
        
        // Access payload
        let payload = msg.payload();
        
        // Make decision based on message content
    }
    ErrorAction::Skip
}
```

## Testing Updates

Update your tests to work with `Message<T>`:

**Before:**
```rust
let producer = TestProducer::new(vec![1, 2, 3]);
let stream = producer.produce();
let items: Vec<i32> = stream.collect().await;
assert_eq!(items, vec![1, 2, 3]);
```

**After:**
```rust
use streamweave::message::wrap_message;

let producer = TestProducer::new(vec![1, 2, 3]);
let stream = producer.produce();
let messages: Vec<Message<i32>> = stream.collect().await;
let payloads: Vec<i32> = messages.iter().map(|m| *m.payload()).collect();
assert_eq!(payloads, vec![1, 2, 3]);
```

## Breaking Changes

1. **All trait associated types changed:**
   - `Producer::Output` is now `Message<T>` instead of `T`
   - `Transformer::Input` and `Transformer::Output` are now `Message<T>` and `Message<U>`
   - `Consumer::Input` is now `Message<T>`

2. **Configuration types changed:**
   - `ProducerConfig<T>` → `ProducerConfig<Message<T>>`
   - `TransformerConfig<T>` → `TransformerConfig<Message<T>>`
   - `ConsumerConfig<T>` → `ConsumerConfig<Message<T>>`

3. **Error contexts include messages:**
   - `ErrorContext<T>` now contains `Option<Message<T>>` instead of `Option<T>`

## Migration Checklist

- [ ] Update `Producer::Output` to `Message<T>`
- [ ] Update `Transformer::Input` and `Transformer::Output` to `Message<T>` and `Message<U>`
- [ ] Update `Consumer::Input` to `Message<T>`
- [ ] Wrap payloads in `Message<T>` when producing
- [ ] Preserve message IDs and metadata through transformations
- [ ] Update configuration types to use `Message<T>`
- [ ] Update error handling to work with `Message<T>`
- [ ] Update tests to work with `Message<T>`
- [ ] Consider using adapters for quick migration
- [ ] Update documentation and examples

## Need Help?

- See the [README.md](README.md) for overview and examples
- See the [message module documentation](https://docs.rs/streamweave/latest/streamweave/message/index.html) for API details
- See the [adapters module documentation](https://docs.rs/streamweave/latest/streamweave/adapters/index.html) for adapter patterns

