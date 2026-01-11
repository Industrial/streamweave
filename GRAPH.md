# StreamWeave Graph Architecture

## 🎯 Why a Graph Architecture?

StreamWeave started with a simple, linear pipeline model that worked well for
basic streaming operations. However, as we added more complex features like
fan-out/fan-in patterns and stateful processing, we
realized we needed a more flexible architecture. The graph model emerged as the
natural solution because:

1. **Natural Representation**: Many real-world data flows are naturally
represented as graphs
2. **Flexibility**: Graphs can represent complex topologies beyond simple linear
pipelines
3. **Scalability**: Efficient in-process execution with zero-copy optimizations
4. **Maintainability**: Clear separation of concerns between routing and
processing
5. **Visualization**: Graphs are easy to visualize and debug

## 🏗️ Core Components

### Node Types

```rust
// Base Node with input/output routing
pub struct Node<I, O> {
    name: String,
    input_router: Box<dyn InputRouter<I>>,
    transformer: Box<dyn Transformer<I, O>>,
    output_router: Box<dyn OutputRouter<O>>,
    config: NodeConfig,
}

// Producer Node (source)
pub struct ProducerNode<O> {
    name: String,
    producer: Box<dyn Producer<O>>,
    output_router: Box<dyn OutputRouter<O>>,
    config: NodeConfig,
}

// Consumer Node (sink)
pub struct ConsumerNode<I> {
    name: String,
    input_router: Box<dyn InputRouter<I>>,
    consumer: Box<dyn Consumer<I>>,
    config: NodeConfig,
}
```

### Router Traits

Routers handle the flow of data between nodes:

```rust
pub trait InputRouter<I> {
    fn route(&mut self, input: I) -> Vec<(String, I)>;  // (port_name, data)
}

pub trait OutputRouter<O> {
    fn route(&mut self, output: O) -> Vec<(String, O)>;  // (port_name, data)
}
```

### Router Implementations

We provide several built-in routing strategies:

```rust
// Round-robin routing
pub struct RoundRobinRouter<T> {
    ports: Vec<String>,
    next_port: usize,
}

// Broadcast to all ports
pub struct BroadcastRouter<T> {
    ports: Vec<String>,
}

// Route based on a key
pub struct KeyBasedRouter<T, K> {
    ports: HashMap<K, String>,
    key_fn: Box<dyn Fn(&T) -> K>,
}

// Merge multiple inputs
pub struct MergeRouter<T> {
    ports: Vec<String>,
    merge_strategy: MergeStrategy,
}
```

## 🎨 Graph Structure

The graph manages nodes and their connections:

```rust
pub struct Graph {
    nodes: HashMap<String, Box<dyn Node>>,
    connections: Vec<Connection>,
}

pub struct Connection {
    source: (String, String),  // (node_name, output_port)
    target: (String, String),  // (node_name, input_port)
    config: ConnectionConfig,
}
```

## 🚀 Example Usage

Here's how to build a complex streaming graph:

```rust
let mut graph = Graph::new();

// Create a producer node with broadcast routing
let producer = ProducerNode::new(
    "source",
    FileProducer::new("input.txt"),
    BroadcastRouter::new(vec!["out1", "out2"]),
);

// Create a processing node with merge and key-based routing
let processor = Node::new(
    "processor",
    MergeRouter::new(vec!["in1", "in2"], MergeStrategy::RoundRobin),
    ProcessTransformer::new(),
    KeyBasedRouter::new(
        |item: &ProcessedItem| item.category.clone(),
        vec![
            ("category1", "out1"),
            ("category2", "out2"),
        ],
    ),
);

// Create a consumer node with round-robin input routing
let consumer = ConsumerNode::new(
    "sink",
    RoundRobinRouter::new(vec!["in1", "in2"]),
    FileConsumer::new("output.txt"),
);

// Add nodes to graph
graph.add_node("source", producer);
graph.add_node("processor", processor);
graph.add_node("sink", consumer);

// Connect nodes
graph.connect(
    ("source", "out1"),
    ("processor", "in1"),
)?;
graph.connect(
    ("source", "out2"),
    ("processor", "in2"),
)?;
graph.connect(
    ("processor", "out1"),
    ("sink", "in1"),
)?;
graph.connect(
    ("processor", "out2"),
    ("sink", "in2"),
)?;

// Run the graph
graph.run().await?;
```

## 🔄 Advanced Features

The graph architecture naturally supports our planned advanced features:

### Stateful Processing

```rust
let stateful_node = Node::new(
    "stateful",
    MergeRouter::new(vec!["in"], MergeStrategy::Ordered),
    StatefulTransformer::new(
        HashMap::new(),
        |state, item| {
            // Update state and return result
            state.entry(item.key).or_insert(0) += 1;
            (state.clone(), item)
        }
    ),
    BroadcastRouter::new(vec!["out"]),
);
```

### Exactly-Once Processing

```rust
let exactly_once_node = Node::new(
    "exactly_once",
    MergeRouter::new(vec!["in"], MergeStrategy::Ordered),
    ExactlyOnceTransformer::new(
        RedisCheckpointStore::new("redis://localhost"),
        |item| async {
            // Process item
            process_item(&item).await
        }
    ),
    BroadcastRouter::new(vec!["out"]),
);
```

### Windowing Operations

```rust
let windowing_node = Node::new(
    "windowing",
    MergeRouter::new(vec!["in"], MergeStrategy::TimeOrdered),
    WindowTransformer::new(
        WindowType::Time(Duration::from_secs(60)),
        |window| {
            // Process window
            process_window(&window)
        }
    ),
    BroadcastRouter::new(vec!["out"]),
);
```

## 🎯 Benefits

1. **Clean Separation of Concerns**
   - Routing logic is separate from transformation logic
   - Each component has a single responsibility
   - Easy to swap out different routing strategies

2. **Flexible Routing**
   - Different routing strategies can be mixed and matched
   - Easy to add new routing strategies
   - Clear interface for routing decisions

3. **Type Safety**
   - Input and output types are clearly defined
   - Routing decisions are type-safe
   - Easy to catch connection mismatches at compile time

4. **Extensibility**
   - Easy to add new node types
   - Easy to add new routing strategies
   - Easy to add new transformation types

## 🚀 Future Directions

1. **Visualization Tools**
   - Graph visualization
   - Real-time monitoring
   - Performance metrics

2. **Advanced Routing**
   - Dynamic routing based on load
   - Circuit breaking
   - Rate limiting

3. **State Management**
   - State persistence
   - State recovery
   - Advanced stateful patterns

## 🤝 Contributing

We welcome contributions to the graph architecture! Areas of particular
interest:

1. New routing strategies
2. Visualization tools
3. State management improvements
4. Performance optimizations
5. Advanced control flow patterns 