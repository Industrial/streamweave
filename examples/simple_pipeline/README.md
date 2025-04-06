# Simple Pipeline Example

This example demonstrates a basic data processing pipeline using StreamWeave with comprehensive error handling. It shows how to:
1. Create a producer that generates a sequence of numbers
2. Transform the data by squaring each number
3. Consume the results by printing them to the console
4. Handle errors at each stage of the pipeline

## How to Run

```bash
# From the project root
cargo run --example simple_pipeline
```

## Expected Output

```
Received value: 1
Received value: 4
Received value: 9
Received value: 16
Received value: 25
Pipeline completed successfully
Consumer info: ComponentInfo { name: "console_printer", type_name: "ConsoleConsumer" }
```

## Error Handling

The example demonstrates different error handling strategies at each stage:

1. **Pipeline Level**: Retry up to 3 times
2. **Producer**: Skip errors and continue
3. **Transformer**: Retry up to 2 times
4. **Consumer**: Stop on first error

If an error occurs, you'll see detailed error information including:
- The error itself
- The context (timestamp, stage, and item)
- The component that failed

## Explanation

The pipeline consists of three components:

1. **Producer**: `RangeProducer` generates numbers from 1 to 5
   - Configured to skip errors and continue processing
2. **Transformer**: `MapTransformer` squares each number
   - Configured to retry failed transformations up to 2 times
3. **Consumer**: `ConsoleConsumer` prints each transformed number
   - Configured to stop on first error
   - Named "console_printer" for better error reporting

The pipeline is built using the `PipelineBuilder` and executed with the `run()` method. The result shows that all numbers were successfully processed and printed to the console, with proper error handling in place. 