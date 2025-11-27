# Common Use Cases

This guide demonstrates common patterns and use cases for StreamWeave.

## Data Transformation Pipeline

Transform data from one format to another:

```rust
use streamweave::prelude::*;

let pipeline = Pipeline::new()
    .with_producer(CsvProducer::new("input.csv"))
    .with_transformer(MapTransformer::new(|row| {
        // Transform CSV row
        row
    }))
    .with_consumer(JsonlConsumer::new("output.jsonl"));
```

## Real-time Data Processing

Process data from Kafka in real-time:

```rust
use streamweave::prelude::*;

let pipeline = Pipeline::new()
    .with_producer(KafkaProducer::new("my-topic"))
    .with_transformer(FilterTransformer::new(|msg| {
        msg.value().len() > 100
    }))
    .with_transformer(BatchTransformer::new(100))
    .with_consumer(DatabaseConsumer::new(connection));
```

## Error Handling Patterns

Handle errors gracefully:

```rust
use streamweave::prelude::*;

let pipeline = Pipeline::new()
    .with_error_strategy(ErrorStrategy::Retry {
        max_attempts: 3,
        backoff: Duration::from_secs(1),
    })
    .with_producer(HttpPollProducer::new(url))
    .with_transformer(RetryTransformer::new())
    .with_consumer(VecConsumer::new());
```

## Windowing Operations

Process data in time-based windows:

```rust
use streamweave::prelude::*;

let pipeline = Pipeline::new()
    .with_producer(KafkaProducer::new("events"))
    .with_transformer(WindowTransformer::new(
        TumblingWindow::new(Duration::from_secs(60))
    ))
    .with_transformer(ReduceTransformer::new(|acc, item| {
        acc + item.value()
    }))
    .with_consumer(ConsoleConsumer::new());
```

## Stateful Processing

Maintain state across stream items:

```rust
use streamweave::prelude::*;

let pipeline = Pipeline::new()
    .with_producer(ArrayProducer::new(data))
    .with_transformer(RunningSumTransformer::new())
    .with_transformer(MovingAverageTransformer::new(10))
    .with_consumer(VecConsumer::new());
```

