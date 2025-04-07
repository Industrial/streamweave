# Advanced Pipeline Example

This example demonstrates several advanced features of the StreamWeave library:

1. **File I/O Operations**: Reading from and writing to files using `FileProducer` and `FileConsumer`
2. **Data Transformation**: Converting string input to integers using `MapTransformer`
3. **Batching**: Grouping items into batches of 3 using `BatchTransformer`
4. **Rate Limiting**: Controlling throughput with `RateLimitTransformer`
5. **Error Handling**: 
   - Using `CircuitBreakerTransformer` to prevent cascading failures
   - Implementing retry logic with `RetryTransformer`
   - Configuring error strategies with `ErrorStrategy`

## How to Run

```bash
cargo run --example advanced_pipeline
```

## Pipeline Flow

1. Reads numbers from an input file (one per line)
2. Converts each line to an integer
3. Groups numbers into batches of 3
4. Rate limits to 2 batches per second
5. Uses circuit breaker to handle failures (opens after 3 failures, resets after 5 seconds)
6. Retries failed operations up to 3 times with 100ms delay between retries
7. Writes the processed batches to an output file

## Error Handling

The example demonstrates several error handling strategies:
- String parsing errors are caught and wrapped in `StreamError`
- The circuit breaker prevents cascading failures
- Retry logic attempts to recover from transient failures
- The file consumer is configured to stop on errors 