# streamweave-visualization

[![Crates.io](https://img.shields.io/crates/v/streamweave-visualization.svg)](https://crates.io/crates/streamweave-visualization)
[![Documentation](https://docs.rs/streamweave-visualization/badge.svg)](https://docs.rs/streamweave-visualization)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Visualization tools for StreamWeave**  
*Visualize pipelines and graphs as DAGs with interactive visualization.*

The `streamweave-visualization` package provides visualization tools for StreamWeave pipelines and graphs. It enables generating DAG representations, exporting to various formats (DOT, HTML, JSON), and serving interactive visualizations.

## ✨ Key Features

- **Pipeline DAG**: Represent pipelines as directed acyclic graphs
- **Graph Visualization**: Visualize graph structures
- **Format Export**: Export to DOT, HTML, JSON formats
- **Interactive Visualization**: Interactive web-based visualization
- **Visualization Server**: Serve visualizations via HTTP
- **Real-Time Updates**: Real-time metrics visualization
- **Debug Support**: Debug visualization support

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-visualization = "0.3.0"
```

## 🚀 Quick Start

### Generate DAG

```rust
use streamweave_visualization::{PipelineDag, DagExporter};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(/* producer */)
    .transformer(/* transformer */)
    .consumer(/* consumer */)
    .build()?;

let dag = PipelineDag::from_pipeline(&pipeline);
```

### Export to DOT

```rust
use streamweave_visualization::{PipelineDag, DagExporter};

let dag = PipelineDag::from_pipeline(&pipeline);
let dot = dag.to_dot();

// Use with Graphviz: dot -Tpng <<< "$dot"
```

### Export to JSON

```rust
use streamweave_visualization::{PipelineDag, DagExporter};

let dag = PipelineDag::from_pipeline(&pipeline);
let json = dag.to_json()?;
```

### Interactive Visualization

```rust
use streamweave_visualization::{PipelineDag, generate_standalone_html};

let dag = PipelineDag::from_pipeline(&pipeline);
let html = generate_standalone_html(&dag);
```

## 📖 API Overview

### PipelineDag

Represents a pipeline as a DAG:

```rust
pub struct PipelineDag {
    pub nodes: Vec<DagNode>,
    pub edges: Vec<DagEdge>,
    pub metadata: HashMap<String, String>,
}
```

**Key Methods:**
- `new()` - Create empty DAG
- `from_pipeline(pipeline)` - Create DAG from pipeline
- `from_graph(graph)` - Create DAG from graph
- `add_node(node)` - Add node to DAG
- `add_edge(edge)` - Add edge to DAG
- `to_dot()` - Export to DOT format
- `to_json()` - Export to JSON format

### DagNode

Represents a node in the DAG:

```rust
pub struct DagNode {
    pub id: String,
    pub kind: NodeKind,
    pub metadata: NodeMetadata,
}
```

### DagEdge

Represents an edge in the DAG:

```rust
pub struct DagEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}
```

### VisualizationServer

Serves visualizations via HTTP:

```rust
pub struct VisualizationServer {
    // Internal state
}
```

**Key Methods:**
- `new(port)` - Create server
- `add_pipeline(name, dag)` - Add pipeline to server
- `start()` - Start server

## 📚 Usage Examples

### Pipeline Visualization

Visualize a pipeline:

```rust
use streamweave_visualization::{PipelineDag, DagExporter};
use streamweave_pipeline::PipelineBuilder;

let pipeline = PipelineBuilder::new()
    .producer(/* producer */)
    .transformer(/* transformer */)
    .consumer(/* consumer */)
    .build()?;

let dag = PipelineDag::from_pipeline(&pipeline);
let dot = dag.to_dot();
println!("{}", dot);
```

### Graph Visualization

Visualize a graph:

```rust
use streamweave_visualization::PipelineDag;
use streamweave_graph::{GraphBuilder, Graph};

let graph = GraphBuilder::new()
    .add_producer(/* producer */)
    .add_transformer(/* transformer */)
    .add_consumer(/* consumer */)
    .build()?;

let dag = PipelineDag::from_graph(&graph);
let json = dag.to_json()?;
```

### Interactive Server

Start visualization server:

```rust
use streamweave_visualization::VisualizationServer;

let server = VisualizationServer::new(8080);
server.add_pipeline("my-pipeline", dag);
server.start().await?;
```

### Standalone HTML

Generate standalone HTML:

```rust
use streamweave_visualization::{PipelineDag, generate_standalone_html};

let dag = PipelineDag::from_pipeline(&pipeline);
let html = generate_standalone_html(&dag);

// Save to file or serve via HTTP
std::fs::write("visualization.html", html)?;
```

### Real-Time Metrics

Visualize real-time metrics:

```rust
use streamweave_visualization::{PipelineMetrics, NodeMetrics};

let metrics = PipelineMetrics::new();
metrics.update_node_metrics("producer_1", NodeMetrics {
    throughput: 100.0,
    latency: Duration::from_millis(10),
});
```

## 🏗️ Architecture

Visualization flow:

```text
Pipeline/Graph ──> PipelineDag ──> DagExporter ──> DOT/HTML/JSON ──> Visualization
```

**Visualization Flow:**
1. Pipeline or graph is converted to DAG
2. DAG is exported to visualization format
3. Visualization is rendered (browser, Graphviz, etc.)
4. Real-time metrics can be overlaid

## 🔧 Configuration

### PipelineDag

- **Nodes**: Pipeline components (producers, transformers, consumers)
- **Edges**: Data flow connections
- **Metadata**: Additional pipeline information

### VisualizationServer

- **Port**: HTTP server port
- **Pipelines**: Registered pipelines to visualize

## 🔍 Error Handling

Visualization errors are handled:

```rust
use streamweave_visualization::PipelineDag;

match PipelineDag::from_pipeline(&pipeline) {
    Ok(dag) => { /* use dag */ }
    Err(e) => { /* handle error */ }
}
```

## ⚡ Performance Considerations

- **DAG Generation**: Efficient DAG generation
- **Format Export**: Fast format conversion
- **Server Performance**: Efficient HTTP server

## 📝 Examples

For more examples, see:
- [Visualization Examples](https://github.com/Industrial/streamweave/tree/main/examples/visualization)
- [DAG Generation Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-visualization` depends on:

- `streamweave` - Core traits
- `streamweave-graph` - Graph structures
- `streamweave-error` - Error handling
- `serde` - Serialization support
- `serde_json` - JSON serialization
- `tokio` - Async runtime

## 🎯 Use Cases

Visualization is used for:

1. **Pipeline Debugging**: Visualize pipeline structure
2. **Documentation**: Generate pipeline diagrams
3. **Monitoring**: Monitor pipeline structure
4. **Development**: Understand pipeline topology
5. **Presentation**: Present pipeline designs

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-visualization)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/visualization)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-graph](../graph/README.md) - Graph structures
- [streamweave-pipeline](../pipeline/README.md) - Pipeline execution

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.

