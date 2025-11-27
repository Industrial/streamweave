# Error Handling Example

This example demonstrates StreamWeave's comprehensive error handling strategies. Error handling is critical for production pipelines, and this example shows when and how to use each strategy.

## Prerequisites

No external dependencies required! This example uses built-in error handling features.

## Quick Start

```bash
# Run stop strategy example
cargo run --example error_handling stop

# Run skip strategy example
cargo run --example error_handling skip

# Run retry strategy example
cargo run --example error_handling retry

# Run custom strategy example
cargo run --example error_handling custom

# Run component-level example
cargo run --example error_handling component
```

## Running Examples

### 1. Stop Strategy

This example demonstrates stopping immediately on errors:

```bash
cargo run --example error_handling stop
```

**What it demonstrates:**
- Pipeline stops immediately on first error
- No partial results are returned after error
- Useful for critical data where errors are unacceptable

**When to use:**
- Data integrity is critical
- Errors indicate system failure
- Partial results are not acceptable
- Financial or safety-critical systems

**Output:**
```
🛑 Setting up Stop strategy example...
🔄 Processing with Stop strategy...
   Input: [1, 2, -1, 4, 5]
   Expected: Pipeline stops at -1, only [2, 4] processed

✅ Stop strategy completed!
📊 Results (2 items):
  1. 2
  2. 4

💡 Stop Strategy:
   - Pipeline stops immediately on first error
   - No partial results after error
   - Use when errors indicate critical failure
   - Best for: Data integrity requirements, critical pipelines
```

### 2. Skip Strategy

This example demonstrates skipping errors and continuing:

```bash
cargo run --example error_handling skip
```

**What it demonstrates:**
- Errors are skipped, processing continues
- Partial results are returned
- Useful for non-critical errors or data cleaning

**When to use:**
- Errors are non-critical
- Data cleaning/filtering scenarios
- Logging and monitoring
- When partial results are acceptable

**Output:**
```
⏭️  Setting up Skip strategy example...
🔄 Processing with Skip strategy...
   Input: [1, 2, -1, 4, -2, 5, 6]
   Expected: Skips -1 and -2, processes [1, 2, 4, 5, 6]

✅ Skip strategy completed!
📊 Results (5 items, errors skipped):
  1. 2
  2. 4
  3. 8
  4. 10
  5. 12

💡 Skip Strategy:
   - Errors are skipped, processing continues
   - Partial results are returned
   - Use when errors are non-critical
   - Best for: Data cleaning, filtering invalid records, logging
```

### 3. Retry Strategy

This example demonstrates automatic retries for transient failures:

```bash
cargo run --example error_handling retry
```

**What it demonstrates:**
- Errors trigger retries up to a maximum count
- Useful for transient failures
- After max retries, falls back to Stop

**When to use:**
- Transient failures (network, timeouts)
- External service calls
- Temporary resource unavailability
- Network-related errors

**Output:**
```
🔄 Setting up Retry strategy example...
🔄 Processing with Retry strategy...
   Input: [1, 2, 3, 4, 5]
   Simulated: Even numbers fail (transient errors)
   Retry limit: 3 (after which it stops)

✅ Retry strategy completed!
📊 Results (3 items after retries):
  1. 2
  2. 6
  3. 10

💡 Retry Strategy:
   - Errors trigger retries up to maximum count
   - Useful for transient failures (network, timeouts)
   - After max retries, falls back to Stop
   - Best for: Network calls, external services, temporary failures
```

### 4. Custom Strategy

This example demonstrates custom error handling logic:

```bash
cargo run --example error_handling custom
```

**What it demonstrates:**
- Custom error handling logic
- Conditional error handling based on error type or context
- Complex error recovery patterns

**When to use:**
- Complex error scenarios
- Conditional error handling
- Different strategies for different error types
- Advanced error recovery

**Output:**
```
🎯 Setting up Custom strategy example...
🔄 Processing with Custom strategy...
   Input: [1, 2, -1, 4, -2, 5, 100, 6]
   Strategy: Skip negatives, Stop on numbers > 50
   Expected: Processes [1, 2, 4, 5], stops at 100

✅ Custom strategy completed!
📊 Results (4 items):
  1. 2
  2. 4
  3. 8
  4. 10

💡 Custom Strategy:
   - Implement custom error handling logic
   - Make decisions based on error type or context
   - Can combine multiple strategies conditionally
   - Best for: Complex error scenarios, conditional recovery
```

### 5. Component-Level Error Handling

This example demonstrates different error strategies per component:

```bash
cargo run --example error_handling component
```

**What it demonstrates:**
- Different error strategies at component level
- Overriding pipeline-level strategy per component
- Fine-grained error control

**When to use:**
- Different error tolerance per pipeline stage
- Component-specific error handling
- Fine-grained control over error behavior
- Mixed error strategies in one pipeline

**Output:**
```
🔧 Setting up Component-level error handling example...
🔄 Processing with component-level strategies...
   Input: [1, 2, -1, 4, 5]
   Transformer1: Skip errors (will skip -1)
   Transformer2: Stop errors (will stop if x > 10)

✅ Component-level error handling completed!
📊 Results (4 items):
  1. 12
  2. 14
  3. 18
  4. 20

💡 Component-Level Error Handling:
   - Each component can have its own error strategy
   - Overrides pipeline-level strategy
   - Allows fine-grained error control
   - Best for: Different error tolerance per stage
```

## Error Strategies Comparison

| Strategy | Behavior | Use Case | Best For |
|----------|----------|----------|----------|
| **Stop** | Stops immediately on error | Critical failures | Data integrity, safety-critical |
| **Skip** | Skips error, continues | Non-critical errors | Data cleaning, logging |
| **Retry** | Retries up to max count | Transient failures | Network calls, external services |
| **Custom** | User-defined logic | Complex scenarios | Conditional handling, advanced recovery |

## Configuration

### Pipeline-Level Error Strategy

```rust
PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip) // Pipeline-level default
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();
```

### Component-Level Error Strategy

```rust
let producer = VecProducer::new(data)
    .with_error_strategy(ErrorStrategy::Skip); // Component-level override

let transformer = MapTransformer::new(|x| Ok(x * 2))
    .with_error_strategy(ErrorStrategy::Stop); // Different strategy
```

### Custom Error Strategy

```rust
let custom_strategy = ErrorStrategy::new_custom(|error: &StreamError<T>| {
    let error_msg = error.source.to_string();
    
    if error_msg.contains("transient") {
        ErrorAction::Retry
    } else if error_msg.contains("critical") {
        ErrorAction::Stop
    } else {
        ErrorAction::Skip
    }
});
```

## Best Practices

1. **Choose Appropriate Strategy**: Match strategy to error type and business requirements
2. **Component-Level Control**: Use component-level strategies for fine-grained control
3. **Custom Logic**: Use custom strategies for complex conditional handling
4. **Error Logging**: Always log errors for monitoring and debugging
5. **Retry Limits**: Set reasonable retry limits to avoid infinite loops
6. **Error Context**: Use error context to make informed decisions in custom handlers

## Error Context

Each error includes rich context:

```rust
pub struct StreamError<T> {
    pub source: Box<dyn Error + Send + Sync>,  // Original error
    pub context: ErrorContext<T>,              // When/where it occurred
    pub component: ComponentInfo,              // Which component
    pub retries: usize,                         // Retry count
}

pub struct ErrorContext<T> {
    pub timestamp: DateTime<Utc>,              // When
    pub item: Option<T>,                       // What item
    pub component_name: String,                // Component name
    pub component_type: String,                // Component type
}
```

## Next Steps

- Explore other StreamWeave examples
- Combine error handling with other features
- Use with advanced transformers for resilience
- See the main StreamWeave documentation for advanced features
