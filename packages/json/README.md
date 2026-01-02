# streamweave-json

JSON producer and consumer for StreamWeave.

This package provides producers and consumers for reading and writing complete JSON documents (as opposed to JSONL/JSON Lines format).

## Features

- Read complete JSON documents from files
- Write items as JSON arrays or single JSON values
- Support for pretty-printing JSON
- Handle JSON arrays as streams of items or as single items
- Comprehensive error handling with configurable strategies

## Usage

### Producer

The `JsonProducer` reads JSON files and can handle:
- JSON objects: yields the object as a single item
- JSON arrays: can yield each element as a separate item (if `array_as_stream` is true)
- JSON primitives: yields the value as a single item

```rust
use streamweave_json::JsonProducer;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
struct Event {
    id: u32,
    message: String,
}

// Read a JSON array and stream each element
let producer = JsonProducer::<Event>::new("events.json");

// Or read the entire array as a single item
let producer = JsonProducer::<Vec<Event>>::new("events.json")
    .with_array_as_stream(false);
```

### Consumer

The `JsonConsumer` writes items to JSON files. By default, it writes all items as a single JSON array.

```rust
use streamweave_json::JsonConsumer;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
struct Event {
    id: u32,
    message: String,
}

// Write items as a JSON array
let consumer = JsonConsumer::<Event>::new("events.json");

// Or write with pretty printing
let consumer = JsonConsumer::<Event>::new("events.json")
    .with_pretty(true);

// Or write only the first item as a single JSON value
let consumer = JsonConsumer::<Event>::new("event.json")
    .with_as_array(false);
```

### Complete Example

```rust,no_run
use streamweave_json::{JsonProducer, JsonConsumer};
use streamweave::{Producer, Consumer};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read users from a JSON array file
    let mut producer = JsonProducer::<User>::new("users.json")
        .with_array_as_stream(true); // Stream each user separately

    // Write users to a pretty-printed JSON array
    let mut consumer = JsonConsumer::<User>::new("output.json")
        .with_pretty(true);

    // Process the stream
    let stream = producer.produce();
    consumer.consume(stream).await;

    Ok(())
}
```

## Configuration

### Producer Configuration

- `with_array_as_stream(bool)`: If true (default), JSON arrays are streamed element by element. If false, the entire array is yielded as a single item.
- `with_error_strategy(ErrorStrategy)`: Configure error handling (Stop, Skip, Retry, Custom).
- `with_name(String)`: Set a name for the producer (useful for logging and metrics).

### Consumer Configuration

- `with_as_array(bool)`: If true (default), all items are written as a JSON array. If false, only the first item is written as a single JSON value.
- `with_pretty(bool)`: If true, JSON is pretty-printed with indentation. Default is false.
- `with_buffer_size(usize)`: Set the buffer size for writing. Default is 8192 bytes.
- `with_error_strategy(ErrorStrategy)`: Configure error handling.
- `with_name(String)`: Set a name for the consumer.

## Differences from JSONL

- **JSON**: Complete JSON documents (objects, arrays, primitives). Can read/write entire documents or stream array elements.
- **JSONL**: Each line is a separate JSON value. Designed for streaming large datasets line by line.

Use JSON when you need to work with complete JSON documents. Use JSONL when you need to process large datasets line by line without loading everything into memory.

## Error Handling

Both producer and consumer support the standard StreamWeave error handling strategies:

- `ErrorStrategy::Stop`: Stop processing on first error (default)
- `ErrorStrategy::Skip`: Skip items that cause errors and continue
- `ErrorStrategy::Retry(n)`: Retry failed operations up to n times
- `ErrorStrategy::Custom(handler)`: Custom error handling logic

## License

This package is part of StreamWeave and is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

