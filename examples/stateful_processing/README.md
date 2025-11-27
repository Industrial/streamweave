# Stateful Processing Example

This example demonstrates how to use StreamWeave's stateful transformers to maintain state across stream items. Stateful processing enables aggregations, analytics, and pattern detection that require remembering previous items.

## Prerequisites

No external dependencies required! This example uses built-in stateful transformers.

## Quick Start

```bash
# Run running sum example
cargo run --example stateful_processing running-sum

# Run moving average example
cargo run --example stateful_processing moving-average

# Run state checkpoint example
cargo run --example stateful_processing checkpoint
```

## Running Examples

### 1. Running Sum Transformer

This example demonstrates cumulative state maintenance:

```bash
cargo run --example stateful_processing running-sum
```

**What it demonstrates:**
- Maintaining cumulative state across stream items
- Running sum calculations (1, 3, 6, 10, 15, ...)
- State persistence across transformations
- State inspection and reset

**Key Features:**
- **Cumulative State**: Each output is the sum of all previous inputs
- **State Inspection**: Can check current state at any time
- **State Reset**: Can reset state to start fresh
- **Thread-Safe**: State is safely shared across concurrent operations

**Output:**
```
🚀 StreamWeave Stateful Processing Example
===========================================

Running: Running Sum Example
---------------------------
📊 Setting up running sum example...
🔄 Processing numbers 1-10 with running sum...
   Input:  [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
   Output: [1, 3, 6, 10, 15, 21, 28, 36, 45, 55]

✅ Running sum completed!
📊 Results:
  Item 1: Running sum = 1
  Item 2: Running sum = 3
  Item 3: Running sum = 6
  ...
```

### 2. Moving Average Transformer

This example demonstrates sliding window state:

```bash
cargo run --example stateful_processing moving-average
```

**What it demonstrates:**
- Sliding window state management
- Configurable window sizes (3 vs 5 items)
- Different window sizes produce different smoothing
- State that maintains a queue of recent values

**Key Features:**
- **Sliding Window**: Maintains a fixed-size window of recent values
- **Configurable Size**: Window size determines responsiveness vs smoothness
- **Automatic Updates**: Oldest values are automatically removed when window is full
- **Smoothing Effect**: Larger windows provide smoother, less noisy output

**Window Size Comparison:**
- **Small Window (3)**: More responsive to changes, shows more variation
- **Large Window (5)**: Smoother output, less sensitive to individual spikes

**Output:**
```
📈 Moving Average with Window Size 3 (responsive):
   Input values: [10.0, 12.0, 11.0, 13.0, ...]
   Moving averages (window=3):
     Item 1: 10.00
     Item 2: 11.00
     Item 3: 11.00
     ...

📈 Moving Average with Window Size 5 (smoother):
   Moving averages (window=5):
     Item 1: 10.00
     Item 2: 11.00
     Item 3: 10.67
     ...
```

### 3. State Checkpoint Example

This example demonstrates state persistence:

```bash
cargo run --example stateful_processing checkpoint
```

**What it demonstrates:**
- State persistence across multiple pipeline runs
- State inspection at different points
- State lifecycle management
- Starting with custom initial state

**Key Features:**
- **State Persistence**: State survives across pipeline runs
- **Initial State**: Can start with a custom initial value
- **State Inspection**: Check state at any point
- **Lifecycle Management**: Understand when state is created, updated, and reset

**Output:**
```
🔄 First pipeline run (starting from 100):
   Results: [101, 103, 106, 110, 115]
   ✅ State after first run: 115

🔄 Second pipeline run (continuing from previous state):
   Results: [121, 128, 136, 145, 155]
   ✅ State after second run: 155
   ✅ State persisted across pipeline runs!
```

## State Management

### State Lifecycle

1. **Initialization**: State is created with an initial value (default or custom)
2. **Update**: State is updated as each item is processed
3. **Persistence**: State persists across multiple pipeline runs
4. **Inspection**: State can be inspected at any time
5. **Reset**: State can be reset to start fresh

### Stateful Transformer API

```rust
// Create with default initial state (0 for RunningSum)
let transformer = RunningSumTransformer::<i32>::new();

// Create with custom initial state
let transformer = RunningSumTransformer::<i32>::with_initial(100);

// Inspect current state
if let Ok(Some(state)) = transformer.state() {
    println!("Current state: {}", state);
}

// Reset state
transformer.reset_state()?;

// Check if state is initialized
if transformer.has_state() {
    println!("State is initialized");
}
```

## Use Cases

### Running Sum
- Cumulative totals (revenue, counts, scores)
- Running balances
- Progressive calculations
- Accumulating metrics

### Moving Average
- Smoothing noisy data
- Trend detection
- Signal processing
- Real-time analytics

### State Checkpointing
- Resumable processing
- Fault tolerance
- Incremental processing
- State recovery

## Best Practices

1. **Choose Appropriate Window Size**: Balance responsiveness vs smoothness
2. **Initialize State Properly**: Use `with_initial()` for non-zero starting values
3. **Reset When Needed**: Reset state between independent processing runs
4. **Inspect State**: Use state inspection for debugging and monitoring
5. **Thread Safety**: State is thread-safe, but be aware of concurrent access patterns

## State vs Stateless Transformers

### Stateful Transformers
- Remember previous items
- Maintain state across items
- Enable aggregations and analytics
- Examples: RunningSum, MovingAverage

### Stateless Transformers
- Process each item independently
- No memory of previous items
- Simpler and faster
- Examples: Map, Filter

## Next Steps

- Explore other StreamWeave examples
- Combine stateful transformers with other components
- Use with windowing operations for time-based aggregations
- See the main StreamWeave documentation for advanced features

