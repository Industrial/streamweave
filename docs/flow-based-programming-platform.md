# StreamWeave as a Flow-Based Programming Platform

## Executive Summary

This document outlines the roadmap and design for transforming StreamWeave's Graph API into a complete Flow-Based Programming (FBP) platform, comparable to Node-RED, n8n, or Apache NiFi in terms of expressivity and control flow capabilities. The focus is on **execution capabilities only** - we do not plan to build a drag-and-drop web UI, but rather provide the underlying primitives that would enable such a UI to be built.

**Goal**: Enable StreamWeave graphs to express complex control flow patterns including conditionals, loops, pattern matching, error handling branches, and other general-purpose programming constructs while maintaining the streaming, async-first architecture.

---

## Table of Contents

1. [Current Capabilities](#current-capabilities)
2. [Missing Control Flow Constructs](#missing-control-flow-constructs)
3. [Design Principles](#design-principles)
4. [Implementation Roadmap](#implementation-roadmap)
5. [Detailed Specifications](#detailed-specifications)
6. [Examples](#examples)
7. [Architecture Considerations](#architecture-considerations)

---

## Current Capabilities

### What StreamWeave Graph API Already Supports

#### 1. Basic Data Flow
- ✅ **Linear pipelines**: Producer → Transformer → Consumer
- ✅ **Fan-out**: One output to multiple consumers (BroadcastRouter, RoundRobinRouter)
- ✅ **Fan-in**: Multiple inputs to one consumer (MergeRouter)
- ✅ **Key-based routing**: Route items based on extracted keys (KeyBasedRouter)

#### 2. Node Types
- ✅ **Producer nodes**: Generate data streams
- ✅ **Transformer nodes**: Transform data streams
- ✅ **Consumer nodes**: Consume data streams

#### 3. Routing Strategies
- ✅ **Broadcast**: Clone items to all output ports
- ✅ **Round-robin**: Distribute items across ports
- ✅ **Key-based**: Route based on key extraction
- ✅ **Merge**: Combine multiple input streams

#### 4. Advanced Features
- ✅ **Subgraphs**: Nested graph structures
- ✅ **Stateful processing**: Stateful transformers
- ✅ **Windowing**: Time-based and count-based windows
- ✅ **Error handling**: Error strategies (Stop, Skip, Retry, Custom)

### Current Limitations

The current Graph API is **data-flow oriented** but lacks **control-flow constructs**:

1. ❌ **No conditional routing**: Cannot route items based on conditions (if/else)
2. ❌ **No pattern matching**: Cannot route based on pattern matching (match/switch)
3. ❌ **No loops**: Cannot iterate over collections or implement while/for loops
4. ❌ **No variables**: No shared state/variables between nodes
5. ❌ **No functions**: No reusable graph components (beyond subgraphs)
6. ❌ **No error branches**: Cannot route errors to different paths
7. ❌ **No delays**: No time-based control flow (beyond timers)
8. ❌ **No triggers**: No event-driven control flow
9. ❌ **No aggregation**: Limited batch processing capabilities
10. ❌ **No synchronization**: Cannot wait for multiple inputs before proceeding

---

## Missing Control Flow Constructs

### 1. Conditional Routing (If/Else)

**Purpose**: Route items to different output ports based on a condition.

**Use Cases**:
- Filter items into "valid" and "invalid" streams
- Route based on data type or value ranges
- Implement business logic branches

**Example Pattern**:
```
Input → [ConditionalRouter] → {true_port, false_port}
```

**Implementation Approach**:
- New router type: `ConditionalRouter<O>`
- Takes a predicate function: `Fn(&O) -> bool`
- Routes to port 0 if true, port 1 if false
- Can support multiple conditions (if-else-if chains)

### 2. Pattern Matching (Match/Switch)

**Purpose**: Route items to different output ports based on pattern matching.

**Use Cases**:
- Route based on enum variants
- Route based on data type
- Route based on value ranges
- Route based on string patterns (regex)

**Example Pattern**:
```
Input → [MatchRouter] → {variant_a, variant_b, variant_c, default}
```

**Implementation Approach**:
- New router type: `MatchRouter<O, P>` where `P: Pattern`
- Takes a pattern matching function: `Fn(&O) -> Option<usize>`
- Returns port index for matched pattern, or None for default
- Can support Rust-style pattern matching

### 3. Loops

**Purpose**: Iterate over collections or implement iterative processing.

**Use Cases**:
- Process each item in a collection
- Implement while loops (repeat until condition)
- Implement for loops (iterate over range)
- Retry logic with iteration

**Example Patterns**:
```
Collection → [ForEachNode] → Process each item
Item → [WhileLoopNode] → Repeat until condition
```

**Implementation Approach**:
- New node type: `ForEachNode<T>` - expands collections into streams
- New node type: `WhileLoopNode<T>` - repeats processing until condition
- New node type: `RangeNode` - generates ranges for iteration
- Integration with existing transformers for loop body

### 4. Variables and Shared State

**Purpose**: Share state between nodes in a graph.

**Use Cases**:
- Accumulate values across nodes
- Share configuration between nodes
- Implement counters and aggregators
- Store intermediate results

**Example Pattern**:
```
[VariableNode: counter = 0] → [IncrementNode] → [UseCounterNode]
```

**Implementation Approach**:
- New node type: `VariableNode<T>` - stores mutable state
- New node type: `ReadVariableNode<T>` - reads variable value
- New node type: `WriteVariableNode<T>` - updates variable value
- Graph-level variable store: `GraphVariables`
- Thread-safe access via `Arc<Mutex<T>>` or `Arc<RwLock<T>>`

### 5. Functions and Subroutines

**Purpose**: Reusable graph components with parameters.

**Use Cases**:
- Reusable processing pipelines
- Parameterized transformations
- Library of common patterns
- Modular graph construction

**Example Pattern**:
```
[FunctionNode: process_user] → [CallFunctionNode: process_user]
```

**Implementation Approach**:
- Extend existing `Subgraph` to support parameters
- New node type: `FunctionNode` - defines function graph
- New node type: `CallFunctionNode` - invokes function with arguments
- Parameter passing via input ports
- Return values via output ports

### 6. Error Handling Branches

**Purpose**: Route errors to different paths than successful items.

**Use Cases**:
- Separate error handling pipeline
- Error logging and monitoring
- Error recovery workflows
- Dead letter queues

**Example Pattern**:
```
[Transformer] → [ErrorBranchRouter] → {success_port, error_port}
```

**Implementation Approach**:
- New router type: `ErrorBranchRouter<O, E>`
- Routes `Result<O, E>` items
- Success items to port 0, errors to port 1
- Integration with existing error handling

### 7. Delays and Timers

**Purpose**: Time-based control flow and delays.

**Use Cases**:
- Rate limiting with delays
- Scheduled processing
- Timeout handling
- Retry with exponential backoff

**Example Pattern**:
```
Item → [DelayNode: 1s] → Process
```

**Implementation Approach**:
- New node type: `DelayNode<T>` - delays items by duration
- New node type: `TimeoutNode<T>` - times out after duration
- New node type: `ScheduleNode` - processes at scheduled times
- Integration with existing `TimerProducer`

### 8. Triggers and Events

**Purpose**: Event-driven control flow.

**Use Cases**:
- Process on external events
- Webhook handling
- File system events
- Database change events

**Example Pattern**:
```
[EventProducer] → [TriggerNode] → Process
```

**Implementation Approach**:
- New node type: `TriggerNode<T>` - waits for trigger event
- New node type: `EventProducer` - generates events
- Integration with existing signal handling
- Support for custom event types

### 9. Aggregation and Batching

**Purpose**: Collect and process items in batches.

**Use Cases**:
- Batch database writes
- Aggregate calculations
- Windowed processing
- Group by operations

**Example Pattern**:
```
Items → [AggregateNode: count] → Result
```

**Implementation Approach**:
- Extend existing `BatchTransformer`
- New node type: `AggregateNode<T, A>` - aggregates items
- New node type: `GroupByNode<T, K>` - groups items by key
- Support for common aggregations (sum, count, avg, min, max)

### 10. Synchronization

**Purpose**: Wait for multiple inputs before proceeding.

**Use Cases**:
- Join multiple streams
- Wait for all inputs
- Barrier synchronization
- Combine parallel processing results

**Example Pattern**:
```
[Input1] ─┐
[Input2] ─┼→ [SyncNode] → Result
[Input3] ─┘
```

**Implementation Approach**:
- New node type: `SyncNode<T>` - waits for all inputs
- New node type: `JoinNode<T1, T2>` - joins two streams
- New node type: `BarrierNode` - synchronization barrier
- Support for different join strategies (inner, outer, left, right)

---

## Design Principles

### 1. Streaming First
All control flow constructs must work with streams, not blocking operations. This means:
- Conditionals route streams, not individual items synchronously
- Loops generate streams, not blocking iterations
- Synchronization uses async primitives

### 2. Type Safety
Maintain Rust's type safety:
- Compile-time validation of connections
- Type-safe pattern matching
- Type-safe variable access

### 3. Composability
All constructs should compose:
- Nest conditionals
- Combine loops with conditionals
- Use functions in loops
- Mix control flow with data flow

### 4. Performance
Maintain high performance:
- Zero-copy where possible
- Efficient routing
- Minimal overhead for control flow

### 5. Backward Compatibility
New constructs should not break existing code:
- Existing routers continue to work
- Existing nodes continue to work
- New constructs are opt-in

---

## Implementation Roadmap

### Phase 1: Foundation (Months 1-2)

**Goal**: Implement basic control flow constructs

1. **ConditionalRouter**
   - Simple if/else routing
   - Single predicate function
   - Two output ports (true/false)

2. **ErrorBranchRouter**
   - Route `Result<T, E>` items
   - Success/error ports
   - Integration with error handling

3. **VariableNode**
   - Basic variable storage
   - Read/write operations
   - Graph-level variable store

**Deliverables**:
- `ConditionalRouter` implementation
- `ErrorBranchRouter` implementation
- `VariableNode` implementation
- Tests and documentation

### Phase 2: Pattern Matching and Loops (Months 3-4)

**Goal**: Add pattern matching and iteration

1. **MatchRouter**
   - Pattern matching router
   - Multiple output ports
   - Default port support

2. **ForEachNode**
   - Iterate over collections
   - Expand collections to streams
   - Nested iteration support

3. **WhileLoopNode**
   - While loop implementation
   - Condition-based iteration
   - Break/continue support

**Deliverables**:
- `MatchRouter` implementation
- `ForEachNode` implementation
- `WhileLoopNode` implementation
- Examples and documentation

### Phase 3: Functions and Advanced Control Flow (Months 5-6)

**Goal**: Add functions and advanced constructs

1. **FunctionNode and CallFunctionNode**
   - Function definition
   - Function invocation
   - Parameter passing

2. **DelayNode and TimeoutNode**
   - Time-based delays
   - Timeout handling
   - Scheduled processing

3. **SyncNode and JoinNode**
   - Stream synchronization
   - Stream joining
   - Barrier synchronization

**Deliverables**:
- Function system implementation
- Time-based control flow
- Synchronization primitives
- Comprehensive examples

### Phase 4: Advanced Features (Months 7-8)

**Goal**: Add advanced features and optimizations

1. **AggregateNode and GroupByNode**
   - Aggregation operations
   - Group by operations
   - Windowed aggregations

2. **TriggerNode and EventProducer**
   - Event-driven processing
   - Custom event types
   - Event filtering

3. **Optimizations**
   - Zero-copy optimizations
   - Performance improvements
   - Memory optimizations

**Deliverables**:
- Aggregation system
- Event system
- Performance optimizations
- Production-ready implementation

---

## Detailed Specifications

### 1. ConditionalRouter

```rust
/// Router that routes items based on a condition (if/else).
pub struct ConditionalRouter<O> {
    /// Predicate function that determines routing
    predicate: Arc<dyn Fn(&O) -> bool + Send + Sync>,
    /// Output ports: [true_port, false_port]
    output_ports: Vec<usize>,
}

impl<O> ConditionalRouter<O>
where
    O: Send + Sync + 'static,
{
    /// Creates a new ConditionalRouter with a predicate function.
    pub fn new<F>(predicate: F) -> Self
    where
        F: Fn(&O) -> bool + Send + Sync + 'static,
    {
        Self {
            predicate: Arc::new(predicate),
            output_ports: vec![0, 1], // true, false
        }
    }
}

impl<O> OutputRouter<O> for ConditionalRouter<O>
where
    O: Send + Sync + Clone + 'static,
{
    async fn route_stream(
        &mut self,
        stream: Pin<Box<dyn Stream<Item = O> + Send>>,
    ) -> Vec<(usize, Pin<Box<dyn Stream<Item = O> + Send>>)> {
        // Create channels for true/false ports
        let (tx_true, rx_true) = tokio::sync::mpsc::channel(16);
        let (tx_false, rx_false) = tokio::sync::mpsc::channel(16);
        
        let predicate = self.predicate.clone();
        let mut input_stream = stream;
        
        // Spawn routing task
        tokio::spawn(async move {
            while let Some(item) = input_stream.next().await {
                if predicate(&item) {
                    let _ = tx_true.send(item).await;
                } else {
                    let _ = tx_false.send(item).await;
                }
            }
        });
        
        vec![
            (0, Box::pin(async_stream::stream! {
                while let Some(item) = rx_true.recv().await {
                    yield item;
                }
            })),
            (1, Box::pin(async_stream::stream! {
                while let Some(item) = rx_false.recv().await {
                    yield item;
                }
            })),
        ]
    }
    
    fn output_ports(&self) -> Vec<usize> {
        self.output_ports.clone()
    }
}
```

### 2. MatchRouter

```rust
/// Pattern for matching items.
pub trait Pattern<T>: Send + Sync {
    /// Returns the port index if pattern matches, None otherwise.
    fn matches(&self, item: &T) -> Option<usize>;
}

/// Router that routes items based on pattern matching.
pub struct MatchRouter<O, P> {
    /// Patterns to match against
    patterns: Vec<P>,
    /// Default port index (used when no pattern matches)
    default_port: Option<usize>,
    /// Output ports
    output_ports: Vec<usize>,
}

impl<O, P> MatchRouter<O, P>
where
    O: Send + Sync + 'static,
    P: Pattern<O>,
{
    /// Creates a new MatchRouter with patterns.
    pub fn new(patterns: Vec<P>, default_port: Option<usize>) -> Self {
        let num_ports = patterns.len() + default_port.is_some() as usize;
        let output_ports: Vec<usize> = (0..num_ports).collect();
        
        Self {
            patterns,
            default_port,
            output_ports,
        }
    }
}

impl<O, P> OutputRouter<O> for MatchRouter<O, P>
where
    O: Send + Sync + Clone + 'static,
    P: Pattern<O> + 'static,
{
    async fn route_stream(
        &mut self,
        stream: Pin<Box<dyn Stream<Item = O> + Send>>,
    ) -> Vec<(usize, Pin<Box<dyn Stream<Item = O> + Send>>)> {
        // Implementation similar to ConditionalRouter but with multiple patterns
        // ...
    }
    
    fn output_ports(&self) -> Vec<usize> {
        self.output_ports.clone()
    }
}

// Example pattern implementations
pub struct EnumVariantPattern {
    variant_index: usize,
    port_index: usize,
}

pub struct RangePattern<T> {
    range: std::ops::Range<T>,
    port_index: usize,
}

pub struct RegexPattern {
    regex: regex::Regex,
    port_index: usize,
}
```

### 3. ForEachNode

```rust
/// Node that iterates over collections, expanding them into streams.
pub struct ForEachNode<T> {
    /// Function to extract collection from item
    extract_collection: Arc<dyn Fn(&T) -> Vec<T> + Send + Sync>,
}

impl<T> ForEachNode<T>
where
    T: Send + Sync + Clone + 'static,
{
    /// Creates a new ForEachNode.
    pub fn new<F>(extract_collection: F) -> Self
    where
        F: Fn(&T) -> Vec<T> + Send + Sync + 'static,
    {
        Self {
            extract_collection: Arc::new(extract_collection),
        }
    }
}

impl<T> Transformer for ForEachNode<T>
where
    T: Send + Sync + Clone + 'static,
{
    type Input = T;
    type Output = T;
    
    async fn transform(
        &mut self,
        input: Self::InputStream,
    ) -> Self::OutputStream {
        let extract = self.extract_collection.clone();
        Box::pin(input.flat_map(move |item| {
            let collection = extract(&item);
            futures::stream::iter(collection)
        }))
    }
}
```

### 4. VariableNode

```rust
/// Graph-level variable store.
pub struct GraphVariables {
    variables: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl GraphVariables {
    pub fn new() -> Self {
        Self {
            variables: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub fn set<T: Send + Sync + 'static>(&self, name: &str, value: T) {
        let mut vars = self.variables.write().unwrap();
        vars.insert(name.to_string(), Box::new(value));
    }
    
    pub fn get<T: Send + Sync + 'static>(&self, name: &str) -> Option<T> {
        let vars = self.variables.read().unwrap();
        vars.get(name)
            .and_then(|v| v.downcast_ref::<T>())
            .map(|v| v.clone())
    }
}

/// Node that reads a variable value.
pub struct ReadVariableNode<T> {
    variable_name: String,
    variables: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
    _phantom: PhantomData<T>,
}

/// Node that writes a variable value.
pub struct WriteVariableNode<T> {
    variable_name: String,
    variables: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
    _phantom: PhantomData<T>,
}
```

### 5. FunctionNode and CallFunctionNode

```rust
/// Defines a reusable function graph.
pub struct FunctionNode {
    name: String,
    graph: Graph,
    input_ports: Vec<String>,
    output_ports: Vec<String>,
}

/// Invokes a function with arguments.
pub struct CallFunctionNode {
    function_name: String,
    function_registry: Arc<RwLock<HashMap<String, FunctionNode>>>,
}
```

### 6. ErrorBranchRouter

```rust
/// Router that routes Result items to success/error ports.
pub struct ErrorBranchRouter<T, E> {
    /// Output ports: [success_port, error_port]
    output_ports: Vec<usize>,
}

impl<T, E> OutputRouter<Result<T, E>> for ErrorBranchRouter<T, E>
where
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + 'static,
{
    async fn route_stream(
        &mut self,
        stream: Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>,
    ) -> Vec<(usize, Pin<Box<dyn Stream<Item = Result<T, E>> + Send>>)> {
        // Route Ok(items) to port 0, Err(items) to port 1
        // ...
    }
}
```

### 7. DelayNode and TimeoutNode

```rust
/// Node that delays items by a duration.
pub struct DelayNode<T> {
    duration: Duration,
}

impl<T> Transformer for DelayNode<T>
where
    T: Send + Sync + 'static,
{
    type Input = T;
    type Output = T;
    
    async fn transform(
        &mut self,
        input: Self::InputStream,
    ) -> Self::OutputStream {
        let duration = self.duration;
        Box::pin(input.then(move |item| async move {
            tokio::time::sleep(duration).await;
            item
        }))
    }
}

/// Node that times out after a duration.
pub struct TimeoutNode<T> {
    timeout: Duration,
}

impl<T> Transformer for TimeoutNode<T>
where
    T: Send + Sync + 'static,
{
    type Input = T;
    type Output = Result<T, TimeoutError>;
    
    async fn transform(
        &mut self,
        input: Self::InputStream,
    ) -> Self::OutputStream {
        let timeout = self.timeout;
        Box::pin(input.then(move |item| async move {
            tokio::time::timeout(timeout, async { item })
                .await
                .map_err(|_| TimeoutError)
        }))
    }
}
```

### 8. SyncNode and JoinNode

```rust
/// Node that waits for all inputs before proceeding.
pub struct SyncNode<T> {
    expected_inputs: usize,
}

impl<T> InputRouter<T> for SyncNode<T>
where
    T: Send + Sync + 'static,
{
    async fn route_streams(
        &mut self,
        streams: Vec<(usize, Pin<Box<dyn Stream<Item = T> + Send>>)>,
    ) -> Pin<Box<dyn Stream<Item = Vec<T>> + Send>> {
        // Collect items from all streams, emit when all have items
        // ...
    }
}

/// Node that joins two streams.
pub struct JoinNode<T1, T2> {
    join_strategy: JoinStrategy,
}

pub enum JoinStrategy {
    Inner,  // Only emit when both streams have items
    Outer,  // Emit when either stream has items
    Left,   // Emit when left stream has items
    Right,  // Emit when right stream has items
}
```

### 9. AggregateNode and GroupByNode

```rust
/// Node that aggregates items.
pub struct AggregateNode<T, A> {
    aggregator: Arc<dyn Aggregator<T, A> + Send + Sync>,
    window_size: Option<usize>,
    window_duration: Option<Duration>,
}

pub trait Aggregator<T, A>: Send + Sync {
    fn init(&self) -> A;
    fn accumulate(&self, acc: &mut A, item: &T);
    fn finalize(&self, acc: A) -> A;
}

/// Node that groups items by key.
pub struct GroupByNode<T, K> {
    key_fn: Arc<dyn Fn(&T) -> K + Send + Sync>,
}

impl<T, K> Transformer for GroupByNode<T, K>
where
    T: Send + Sync + Clone + 'static,
    K: Hash + Eq + Clone + Send + Sync + 'static,
{
    type Input = T;
    type Output = (K, Vec<T>);
    
    async fn transform(
        &mut self,
        input: Self::InputStream,
    ) -> Self::OutputStream {
        // Group items by key
        // ...
    }
}
```

---

## Examples

### Example 1: Conditional Routing (If/Else)

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::ConditionalRouter;

let mut graph = GraphBuilder::new();

// Create producer
let producer = graph.add_producer_node(
    "source",
    ArrayProducer::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
)?;

// Create conditional router (even/odd)
let router = ConditionalRouter::new(|x: &i32| *x % 2 == 0);
let conditional = graph.add_transformer_node(
    "conditional",
    IdentityTransformer::new(),
    router,
)?;

// Connect producer to conditional
graph.connect(("source", 0), ("conditional", 0))?;

// Connect conditional outputs
graph.connect(("conditional", 0), ("even_consumer", 0))?;  // even numbers
graph.connect(("conditional", 1), ("odd_consumer", 0))?;  // odd numbers

graph.run().await?;
```

### Example 2: Pattern Matching (Match)

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::{MatchRouter, RangePattern};

#[derive(Clone, Debug)]
enum Status {
    Low,
    Medium,
    High,
    Critical,
}

// Create match router with range patterns
let patterns = vec![
    RangePattern::new(0..50, 0),      // Low: 0-49
    RangePattern::new(50..80, 1),     // Medium: 50-79
    RangePattern::new(80..95, 2),     // High: 80-94
    RangePattern::new(95..100, 3),    // Critical: 95-99
];
let router = MatchRouter::new(patterns, Some(0)); // default to Low

let match_node = graph.add_transformer_node(
    "match_status",
    IdentityTransformer::new(),
    router,
)?;
```

### Example 3: ForEach Loop

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::ForEachNode;

// Process each item in a collection
let for_each = ForEachNode::new(|item: &Vec<i32>| {
    item.clone()  // Extract collection
});

let for_each_node = graph.add_transformer_node(
    "for_each",
    for_each,
    BroadcastRouter::new(vec![0]),
)?;

// Now each individual item in the collection is processed separately
```

### Example 4: Variables

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::{VariableNode, ReadVariableNode, WriteVariableNode};

let mut graph = GraphBuilder::new()
    .with_variables(GraphVariables::new());

// Initialize counter variable
graph.set_variable("counter", 0i32)?;

// Read counter
let read_counter = graph.add_transformer_node(
    "read_counter",
    ReadVariableNode::new("counter"),
    BroadcastRouter::new(vec![0]),
)?;

// Increment and write back
let increment = graph.add_transformer_node(
    "increment",
    MapTransformer::new(|x: i32| x + 1),
    BroadcastRouter::new(vec![0]),
)?;

let write_counter = graph.add_transformer_node(
    "write_counter",
    WriteVariableNode::new("counter"),
    BroadcastRouter::new(vec![0]),
)?;

graph.connect(("read_counter", 0), ("increment", 0))?;
graph.connect(("increment", 0), ("write_counter", 0))?;
```

### Example 5: Error Handling Branches

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::ErrorBranchRouter;

// Transformer that may fail
let transformer = graph.add_transformer_node(
    "process",
    MapTransformer::new(|x: i32| -> Result<i32, String> {
        if x < 0 {
            Err("Negative number".to_string())
        } else {
            Ok(x * 2)
        }
    }),
    ErrorBranchRouter::new(),
)?;

// Route success to normal processing
graph.connect(("process", 0), ("success_handler", 0))?;

// Route errors to error handler
graph.connect(("process", 1), ("error_handler", 0))?;
```

### Example 6: Function Call

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::{FunctionNode, CallFunctionNode};

// Define a function
let mut function_graph = GraphBuilder::new();
// ... build function graph ...
let function = FunctionNode::new(
    "process_user",
    function_graph,
    vec!["user_id".to_string()],
    vec!["result".to_string()],
);

// Register function
graph.register_function(function)?;

// Call function
let call = graph.add_transformer_node(
    "call_process_user",
    CallFunctionNode::new("process_user"),
    BroadcastRouter::new(vec![0]),
)?;
```

### Example 7: Synchronization

```rust
use streamweave::graph::*;
use streamweave::graph::control_flow::SyncNode;

// Wait for all inputs before proceeding
let sync = graph.add_transformer_node(
    "sync",
    SyncNode::new(3), // Wait for 3 inputs
    BroadcastRouter::new(vec![0]),
)?;

// Connect multiple inputs
graph.connect(("input1", 0), ("sync", 0))?;
graph.connect(("input2", 0), ("sync", 1))?;
graph.connect(("input3", 0), ("sync", 2))?;

// Process only when all inputs are ready
graph.connect(("sync", 0), ("process", 0))?;
```

### Example 8: Complex Control Flow

```rust
// Combine multiple constructs
let mut graph = GraphBuilder::new();

// 1. Read input
let input = graph.add_producer_node("input", ...)?;

// 2. Conditional: valid/invalid
let validate = graph.add_transformer_node(
    "validate",
    IdentityTransformer::new(),
    ConditionalRouter::new(|x: &Data| x.is_valid()),
)?;

// 3. For valid items: process each field
let for_each = graph.add_transformer_node(
    "for_each_field",
    ForEachNode::new(|x: &Data| x.fields.clone()),
    BroadcastRouter::new(vec![0]),
)?;

// 4. Match on field type
let match_type = graph.add_transformer_node(
    "match_type",
    IdentityTransformer::new(),
    MatchRouter::new(/* patterns */),
)?;

// 5. Aggregate results
let aggregate = graph.add_transformer_node(
    "aggregate",
    AggregateNode::new(SumAggregator::new()),
    BroadcastRouter::new(vec![0]),
)?;

// 6. Write variable
let write_result = graph.add_transformer_node(
    "write_result",
    WriteVariableNode::new("result"),
    BroadcastRouter::new(vec![0]),
)?;

// Connect everything
graph.connect(("input", 0), ("validate", 0))?;
graph.connect(("validate", 0), ("for_each_field", 0))?;  // valid path
graph.connect(("validate", 1), ("error_handler", 0))?;  // invalid path
graph.connect(("for_each_field", 0), ("match_type", 0))?;
graph.connect(("match_type", 0), ("aggregate", 0))?;
graph.connect(("aggregate", 0), ("write_result", 0))?;

graph.run().await?;
```

---

## Architecture Considerations

### 1. Type System Integration

**Challenge**: Control flow constructs need to work with Rust's type system while maintaining flexibility.

**Solution**:
- Use generic types with trait bounds
- Leverage Rust's pattern matching where possible
- Use `Any` trait for variables (with type erasure)
- Provide type-safe wrappers for common patterns

### 2. Performance

**Challenge**: Control flow constructs should not significantly impact performance.

**Solution**:
- Use zero-copy routing where possible
- Minimize allocations in hot paths
- Use efficient data structures (e.g., `HashMap` for variables)
- Profile and optimize critical paths

### 3. Error Handling

**Challenge**: Control flow constructs need robust error handling.

**Solution**:
- Integrate with existing error handling system
- Provide error branches for all constructs
- Support error propagation through control flow
- Allow error recovery in loops and conditionals

### 4. State Management

**Challenge**: Variables and state need to be thread-safe and efficient.

**Solution**:
- Use `Arc<RwLock<T>>` for shared state
- Provide atomic operations where possible
- Support state persistence for in-process execution
- Document state access patterns

### 5. Composability

**Challenge**: All constructs must compose well together.

**Solution**:
- Consistent API design across all constructs
- Support nesting (e.g., conditionals in loops)
- Clear separation of concerns
- Reusable components

### 6. Testing

**Challenge**: Control flow constructs need comprehensive testing.

**Solution**:
- Unit tests for each construct
- Integration tests for composed flows
- Property-based tests for correctness
- Performance benchmarks

### 7. Documentation

**Challenge**: Complex control flow needs clear documentation.

**Solution**:
- Comprehensive API documentation
- Usage examples for each construct
- Patterns and best practices
- Migration guides from other FBP platforms

---

## Comparison with Other FBP Platforms

### Node-RED

**Similarities**:
- Flow-based programming model
- Node-based architecture
- Event-driven processing

**Differences**:
- StreamWeave: Type-safe, compile-time validation
- Node-RED: JavaScript-based, runtime validation
- StreamWeave: Zero-copy optimizations
- Node-RED: Higher-level abstractions

**Missing in StreamWeave**:
- Visual editor (intentionally not planned)
- HTTP request/response nodes (partially exists)
- Dashboard nodes (not planned)

### n8n

**Similarities**:
- Workflow-based processing
- Node-based architecture
- Error handling

**Differences**:
- StreamWeave: Streaming-first
- n8n: Request/response model
- StreamWeave: Rust performance
- n8n: Higher-level integrations

**Missing in StreamWeave**:
- Pre-built integrations (intentionally minimal)
- UI components (not planned)
- Workflow scheduling (can be added)

### Apache NiFi

**Similarities**:
- Data flow programming
- Processor-based architecture
- Flow control

**Differences**:
- StreamWeave: Type-safe
- NiFi: Java-based, more mature
- StreamWeave: Better performance potential
- NiFi: More processors out of the box

**Missing in StreamWeave**:
- Visual editor (not planned)
- Pre-built processors (intentionally minimal)
- Enterprise features (can be added)

---

## Conclusion

Transforming StreamWeave into a complete FBP platform requires adding control flow constructs while maintaining the streaming, async-first architecture. The roadmap outlined in this document provides a clear path to achieving this goal.

**Key Takeaways**:
1. **Control flow is missing**: Current Graph API lacks if/else, loops, pattern matching, etc.
2. **Implementation is feasible**: All constructs can be built using existing primitives
3. **Type safety is maintained**: Rust's type system enables safe control flow
4. **Performance is preserved**: Zero-copy and efficient routing remain priorities
5. **Composability is key**: All constructs must work together seamlessly

**Next Steps**:
1. Implement Phase 1 constructs (ConditionalRouter, ErrorBranchRouter, VariableNode)
2. Gather feedback from users
3. Iterate on design based on real-world usage
4. Continue with subsequent phases

This document serves as a living specification that will evolve as implementation progresses and user feedback is incorporated.

