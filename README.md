# StreamWeave

[![Crates.io](https://img.shields.io/crates/v/streamweave.svg)](https://crates.io/crates/streamweave)
[![Documentation](https://docs.rs/streamweave/badge.svg)](https://docs.rs/streamweave)
[![CI](https://github.com/Industrial/streamweave/workflows/CI/badge.svg)](https://github.com/Industrial/streamweave/actions)
[![codecov](https://codecov.io/gh/Industrial/streamweave/graph/badge.svg)](https://codecov.io/gh/Industrial/streamweave)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Composable, async, stream-first computation in pure Rust**  
*Build fully composable, async data processing graphs using a declarative API.*

StreamWeave is a general-purpose Rust framework built around the concept of
**streaming data**, with a focus on simplicity, composability, and performance.

**High-Performance Memory Management:** Achieves optimal performance through advanced memory pooling techniques, including buffer pooling and string interning, significantly reducing allocation overhead and improving cache locality. This results in ultra-low latency and maximum throughput.

## ✨ Key Features

- Pure Rust API with zero-cost abstractions
- Full async/await compatibility via `futures::Stream`
- **Graph-based API** for building data processing topologies
- **`graph!` macro** for declarative graph construction with minimal syntax (80-90% less boilerplate)
- **Flow-Based Programming (FBP)** patterns with type-safe routing
- Support for fan-in patterns (multiple sources → one target)
- Comprehensive error handling system with multiple strategies
- Code-as-configuration — no external DSLs
- Extensive package ecosystem for I/O, transformations, and integrations

## 📦 Core Concepts

StreamWeave uses a **graph-based architecture** where computation is organized as nodes connected by edges:

| Component       | Description                                |
| --------------- | ------------------------------------------ |
| **Nodes**       | Processing units that consume and produce streams |
| **Edges**       | Connections between node ports that route data |
| **Graphs**      | Collections of nodes and edges forming a processing topology |

Nodes can have:
- **Input ports**: Receive data from upstream nodes or external sources
- **Output ports**: Send data to downstream nodes or external consumers
- **Zero or more inputs/outputs**: Nodes can be sources (no inputs), sinks (no outputs), or transforms (both)

All data flows as `Arc<dyn Any + Send + Sync>` for zero-copy efficiency, with explicit port naming for clarity and type safety.

## 🚀 Quick Start

### Installation

Add StreamWeave to your `Cargo.toml`:

```toml
[dependencies]
streamweave = "0.9.0"
```

### Basic Example: Sum of Squares of Even Numbers

This example demonstrates the Graph API by implementing a classic algorithm: generating numbers 1-10, filtering even numbers, squaring them, and summing the results.

#### Using the Traditional Graph API

```rust
use std::any::Any;
use std::sync::Arc;
use streamweave::edge::Edge;
use streamweave::graph::Graph;
use streamweave::nodes::aggregation::SumNode;
use streamweave::nodes::filter_node::{FilterNode, filter_config};
use streamweave::nodes::map_node::{MapNode, map_config};
use streamweave::nodes::range_node::RangeNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels for external I/O
    let (range_start_tx, range_start_rx) = mpsc::channel(1);
    let (range_end_tx, range_end_rx) = mpsc::channel(1);
    let (range_step_tx, range_step_rx) = mpsc::channel(1);
    let (result_tx, mut result_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

    // Build the graph
    let mut graph = Graph::new("sum_of_squares".to_string());

    // Add nodes
    graph.add_node("range".to_string(), Box::new(RangeNode::new("range".to_string())))?;
    graph.add_node("filter".to_string(), Box::new(FilterNode::new("filter".to_string())))?;
    graph.add_node("square".to_string(), Box::new(MapNode::new("square".to_string())))?;
    graph.add_node("sum".to_string(), Box::new(SumNode::new("sum".to_string())))?;

    // Connect nodes with edges
    graph.add_edge(Edge {
        source_node: "range".to_string(),
        source_port: "out".to_string(),
        target_node: "filter".to_string(),
        target_port: "in".to_string(),
    })?;
    graph.add_edge(Edge {
        source_node: "filter".to_string(),
        source_port: "out".to_string(),
        target_node: "square".to_string(),
        target_port: "in".to_string(),
    })?;
    graph.add_edge(Edge {
        source_node: "square".to_string(),
        source_port: "out".to_string(),
        target_node: "sum".to_string(),
        target_port: "in".to_string(),
    })?;

    // Expose input ports for range configuration
    graph.expose_input_port("range", "start", "start")?;
    graph.expose_input_port("range", "end", "end")?;
    graph.expose_input_port("range", "step", "step")?;
    
    // Expose output port for final result
    graph.expose_output_port("sum", "out", "result")?;

    // Connect external channels
    graph.connect_input_channel("start", range_start_rx)?;
    graph.connect_input_channel("end", range_end_rx)?;
    graph.connect_input_channel("step", range_step_rx)?;
    graph.connect_output_channel("result", result_tx)?;

    // Send range configuration
    range_start_tx.send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>).await?;
    range_end_tx.send(Arc::new(11i32) as Arc<dyn Any + Send + Sync>).await?;
    range_step_tx.send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>).await?;

    // Configure filter to keep only even numbers
    let (filter_config_tx, filter_config_rx) = mpsc::channel(1);
    graph.expose_input_port("filter", "configuration", "filter_config")?;
    graph.connect_input_channel("filter_config", filter_config_rx)?;
    
    // Configure map to square each number  
    let (square_config_tx, square_config_rx) = mpsc::channel(1);
    graph.expose_input_port("square", "configuration", "square_config")?;
    graph.connect_input_channel("square_config", square_config_rx)?;
    
    // Send configurations
    filter_config_tx.send(Arc::new(filter_config(|value| async move {
        if let Ok(arc_i32) = value.downcast::<i32>() {
            Ok(*arc_i32 % 2 == 0)
        } else {
            Err("Expected i32".to_string())
        }
    })) as Arc<dyn Any + Send + Sync>).await?;
    
    square_config_tx.send(Arc::new(map_config(|value| async move {
        if let Ok(arc_i32) = value.downcast::<i32>() {
            Ok(Arc::new(*arc_i32 * *arc_i32) as Arc<dyn Any + Send + Sync>)
        } else {
            Err("Expected i32".to_string())
        }
    })) as Arc<dyn Any + Send + Sync>).await?;

    // Execute the graph
    graph.execute().await?;

    // Wait for result
    if let Some(result) = result_rx.recv().await {
        if let Ok(sum) = result.downcast::<i32>() {
            println!("Sum of squares of even numbers (2² + 4² + 6² + 8² + 10²): {}", *sum);
            // Expected: 4 + 16 + 36 + 64 + 100 = 220
        }
    }

    graph.wait_for_completion().await?;
    Ok(())
}
```

#### Using the `graph!` Macro (Minimal Syntax)

The `graph!` macro provides a declarative syntax that reduces boilerplate significantly:

```rust
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::aggregation::SumNode;
use streamweave::nodes::filter_node::FilterNode;
use streamweave::nodes::map_node::MapNode;
use streamweave::nodes::range_node::RangeNode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build graph with minimal syntax
    let mut graph: Graph = graph! {
        range: RangeNode::new("range".to_string()),
        filter: FilterNode::new("filter".to_string()),
        square: MapNode::new("square".to_string()),
        sum: SumNode::new("sum".to_string()),
        ; range.out => filter.in,
          filter.out => square.in,
          square.out => sum.in,
          sum.out => graph.result
    };
    
    // Expose input ports and connect channels (same as traditional API)
    // ... (external I/O setup remains the same)
    
    // Execute the graph
    Graph::execute(&mut graph).await?;
    graph.wait_for_completion().await?;
    Ok(())
}
```

**Key Benefits of `graph!` Macro:**
- **80-90% less boilerplate** - Connections are declared declaratively
- **Visual clarity** - Graph structure is immediately visible
- **Type safety** - Compile-time validation of node names and connections
- **Explicit ports** - All connections require explicit port names (no defaults)

For more `graph!` macro examples, see:
- `examples/graph_macro_simple.rs` - Simple linear pipeline
- `examples/graph_macro_fan_patterns.rs` - Fan-in patterns
- `examples/graph_macro_io.rs` - Graph I/O patterns

### Graph Visualization

The following Mermaid diagram shows the graph structure with nodes that have multiple inputs and outputs:

```mermaid
graph LR
    Start[Range Config<br/>start: 1<br/>end: 11<br/>step: 1] -->|start| Range[RangeNode<br/>Inputs: start, end, step<br/>Outputs: out]
    Start -->|end| Range
    Start -->|step| Range
    
    Range -->|out| Filter[FilterNode<br/>Inputs: configuration, in<br/>Outputs: out, error]
    FilterConfig[Filter Config<br/>even numbers] -->|configuration| Filter
    
    Filter -->|out| Square[MapNode<br/>Inputs: configuration, in<br/>Outputs: out, error]
    SquareConfig[Map Config<br/>square function] -->|configuration| Square
    
    Square -->|out| Sum[SumNode<br/>Inputs: configuration, in<br/>Outputs: out, error]
    
    Sum -->|out| Result[Result: 220]
    
    style Range fill:#e1f5ff
    style Filter fill:#fff4e1
    style Square fill:#fff4e1
    style Sum fill:#e8f5e9
    style Result fill:#f3e5f5
```

This graph demonstrates:
- **Multiple inputs**: RangeNode receives `start`, `end`, and `step` from separate sources
- **Linear processing**: Data flows through Range → Filter → Square → Sum
- **Configuration ports**: FilterNode and MapNode receive configuration on separate ports
- **Multiple outputs**: Each node has both `out` and `error` ports (error ports not shown for clarity)
- **Graph I/O**: External configuration and results flow through exposed graph ports

For more examples and detailed documentation, see the [package documentation](#-packages) below.

## 📦 Node Modules

StreamWeave is organized as a monorepo with 13 core node modules, each providing specific functionality. Each module provides examples and API reference.

### Core Node Modules

StreamWeave's core functionality is built around a flexible node system. These modules provide the fundamental building blocks for data processing, including:

-   **Advanced Node Operations:** `advanced/` (e.g., `Break`, `Continue`, `Repeat`, `Retry`, `Switch`, `TryCatch`)
-   **Aggregation Nodes:** `aggregation/` (e.g., `Average`, `Count`, `Max`, `Min`, `Sum`)
-   **Arithmetic Nodes:** `arithmetic/` (e.g., `Add`, `Divide`, `Modulo`, `Multiply`, `Power`, `Subtract`)
-   **Array Nodes:** `array/` (e.g., `Concat`, `Contains`, `Flatten`, `Index`, `Length`, `Reverse`, `Slice`, `Sort`, `Split`, `Unique`)
-   **Boolean Logic Nodes:** `boolean_logic/` (e.g., `And`, `Nand`, `Nor`, `Not`, `Or`, `Xor`)
-   **Comparison Nodes:** `comparison/` (e.g., `Equal`, `GreaterThan`, `LessThan`, `NotEqual`)
-   **Math Nodes:** `math/` (e.g., `Abs`, `Ceil`, `Exp`, `Floor`, `Log`, `Max`, `Min`, `Round`, `Sqrt`)
-   **Object Nodes:** `object/` (e.g., `HasProperty`, `Keys`, `Merge`, `Property`, `SetProperty`, `Size`)
-   **Reduction Nodes:** `reduction/` (e.g., `Aggregate`, `GroupBy`, `Reduce`, `Scan`)
-   **Stream Processing Nodes:** `stream/` (e.g., `Buffer`, `Debounce`, `Distinct`, `Filter`, `Map`, `Merge`, `Sample`, `Skip`, `Take`, `Throttle`, `Window`, `Zip`)
-   **String Manipulation Nodes:** `string/` (e.g., `Append`, `Capitalize`, `CharAt`, `Concat`, `Contains`, `EndsWith`, `Format`, `IndexOf`, `Join`, `Length`, `Lowercase`, `Match`, `Prepend`, `Replace`, `Slice`, `Split`, `StartsWith`, `Trim`, `Uppercase`)
-   **Time-based Nodes:** `time/` (e.g., `CurrentTime`, `Delay`, `FormatTime`, `ParseTime`, `Timeout`, `Timer`, `Timestamp`)
-   **Type Operation Nodes:** `type_ops/` (e.g., `IsArray`, `IsBoolean`, `IsFloat`, `IsInt`, `IsNull`, `IsNumber`, `IsObject`, `IsString`, `ToArray`, `ToBoolean`, `ToFloat`, `ToInt`, `ToNumber`, `ToString`, `TypeOf`)
-   **Variable Nodes:** `variable/` (e.g., `ReadVariable`, `WriteVariable`)
-   **Flow Control Nodes:** `while_loop_node.rs` (e.g., `WhileLoop`)


For detailed examples and usage of each node, please refer to the [examples directory](examples/nodes/).

## ⏱ Time Semantics

StreamWeave supports two time models for streaming data:

| | **Event time** | **Processing time** |
|--|----------------|---------------------|
| **Definition** | When the event actually occurred (from payload) | When the system processes the event |
| **Ordering** | By event time (handles late/out-of-order data) | By arrival order |
| **Use case** | Analytics, windowing, correct aggregations | Simple pipelines, low-latency processing |

Implement the [`HasEventTime`](https://docs.rs/streamweave/*/streamweave/time/trait.HasEventTime.html) trait for payloads that carry event time. See [docs/event-time-semantics.md](docs/event-time-semantics.md) for details.

## 📊 Progress tracking (watermarks)

Use [`execute_with_progress`](https://docs.rs/streamweave/*/streamweave/graph/struct.Graph.html#method.execute_with_progress) to get a [`ProgressHandle`](https://docs.rs/streamweave/*/streamweave/time/struct.ProgressHandle.html) that reports the minimum logical timestamp completed at sinks. Call `frontier()`, `less_than(t)`, or `less_equal(t)` to observe progress. For graphs with multiple sinks, use [`execute_with_progress_per_sink`](https://docs.rs/streamweave/*/streamweave/graph/struct.Graph.html#method.execute_with_progress_per_sink) so that graph-level progress is the **minimum** over per-sink frontiers ("all sinks have completed up to T"). See [docs/progress-tracking.md](docs/progress-tracking.md).

## 🔁 Determinism

By default, nodes run **concurrently**; ordering between nodes and across multiple inputs is not guaranteed. For reproducible runs, use a **deterministic execution mode** (when available): single-task, topological order, or logical-time ordering. The determinism contract: output order of a node is deterministic given the order of items on its input ports and the node's internal logic. Avoid shared mutable state and non-deterministic constructs (e.g. `rand::random()`, unordered fan-in) in nodes that need reproducibility. See [docs/deterministic-execution.md](docs/deterministic-execution.md) and [docs/architecture.md](docs/architecture.md#determinism).

## ✅ Exactly-once state

Stateful nodes that need correct semantics after replay or recovery should use the **exactly-once state contract**: state is keyed, every update carries a **version** (e.g. `LogicalTime`), and `put(key, value, version)` is **idempotent** (applying the same triple again has no effect). Implement [`ExactlyOnceStateBackend`](https://docs.rs/streamweave/*/streamweave/state/trait.ExactlyOnceStateBackend.html) for custom state stores. See [docs/exactly-once-state.md](docs/exactly-once-state.md).

## Scope and limitations

StreamWeave runs in a **single process**. It does not provide distributed execution or distributed fault tolerance. For scale-out or HA, run multiple processes and use external coordination (e.g. Kafka consumer groups, external state stores). See [docs/architecture.md](docs/architecture.md) and [docs/scope-in-process-no-distributed-fault-tolerance.md](docs/scope-in-process-no-distributed-fault-tolerance.md).

## 📚 Documentation

- [API Documentation](https://docs.rs/streamweave) - Full API reference on docs.rs
- [Local Documentation](target/doc/streamweave/index.html) - Generated with rustdoc (run `./bin/docs`)
- [Graph API Guide](GRAPH.md) - Advanced graph patterns, routing strategies, and Flow-Based Programming
- [Getting Started Guide](docs/getting_started.md)
- [Architecture Overview](docs/architecture.md)
- [Common Use Cases](docs/guides/common_use_cases.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Contributing Guide](CONTRIBUTING.md)

## 📖 Examples

StreamWeave includes comprehensive examples demonstrating all major features. See the [examples directory](examples/) for:

- **Graph Macro Examples:**
  - `graph_macro_simple.rs` - Simple linear pipeline with `graph!` macro
  - `graph_macro_fan_patterns.rs` - Fan-in patterns (fan-out not supported)
  - `graph_macro_io.rs` - Graph I/O with values and without values
- Integration examples (Kafka, Redis, Database, HTTP)
- File format examples (CSV, JSONL, Parquet)
- Processing examples (Stateful, Error Handling, Windowing)
- Visualization examples
- Graph API examples

Run any example with:
```bash
cargo run --example <example_name> --features <required_features>
```

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [Creative Commons Attribution-ShareAlike 4.0 International License](https://creativecommons.org/licenses/by-sa/4.0/).

See \[LICENSE\](LICENSE) for details.

## 🙏 Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- HTTP support powered by [Axum](https://github.com/tokio-rs/axum)
- Inspired by reactive programming patterns and stream processing frameworks
