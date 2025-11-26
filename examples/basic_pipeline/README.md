# Basic Pipeline Example

This example demonstrates the most basic usage of StreamWeave by creating a simple pipeline that:
1. Produces a sequence of numbers
2. Transforms them by doubling each number
3. Consumes them by printing to the console

## Running the Example

```bash
cargo run --example basic_pipeline
```

## Expected Output

```
2
4
6
8
10
```

## Code Explanation

The example creates a pipeline with three components:
1. A `RangeProducer` that generates numbers from 1 to 5
2. A `MapTransformer` that doubles each number
3. A `ConsoleConsumer` that prints each number to the console

This demonstrates the basic flow of data through a StreamWeave pipeline:
- Producer → Transformer → Consumer 