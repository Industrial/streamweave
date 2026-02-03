# Graph Macro Migration Guide

This guide helps you migrate from the traditional Graph API to the new `graph!` macro for declarative graph construction.

## Overview

The `graph!` macro provides a concise, declarative syntax that reduces boilerplate by 80-90% while maintaining the same functionality as the traditional Graph API.

## Before and After Comparison

### Traditional Graph API

```rust
use streamweave::edge::Edge;
use streamweave::graph::Graph;

let mut graph = Graph::new("my_graph".to_string());

// Add nodes
graph.add_node("producer".to_string(), Box::new(producer_node))?;
graph.add_node("transform".to_string(), Box::new(transform_node))?;
graph.add_node("sink".to_string(), Box::new(sink_node))?;

// Connect nodes
graph.add_edge(Edge {
    source_node: "producer".to_string(),
    source_port: "out".to_string(),
    target_node: "transform".to_string(),
    target_port: "in".to_string(),
})?;
graph.add_edge(Edge {
    source_node: "transform".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
})?;

// Expose graph I/O
graph.expose_input_port("producer", "in", "input")?;
graph.expose_output_port("sink", "out", "output")?;
```

### Using `graph!` Macro

```rust
use streamweave::graph;

let mut graph: Graph = graph! {
    producer: producer_node,
    transform: transform_node,
    sink: sink_node,
    ; producer.out => transform.in,
      transform.out => sink.in,
      graph.input => producer.in,
      sink.out => graph.output
};
```

**Benefits:**
- **80-90% less code** - Connections declared declaratively
- **Visual clarity** - Graph structure immediately visible
- **Type safety** - Compile-time validation
- **Less error-prone** - Fewer string conversions and manual edge construction

## Migration Steps

### Step 1: Add the Macro Import

```rust
use streamweave::graph;  // Import the macro
```

### Step 2: Convert Node Addition

**Before:**
```rust
graph.add_node("producer".to_string(), Box::new(producer_node))?;
```

**After:**
```rust
graph! {
    producer: producer_node,
    // ...
}
```

**Key Changes:**
- Node name becomes an identifier (no quotes)
- Node instance is passed directly (no `Box::new()` needed)
- Multiple nodes separated by commas

### Step 3: Convert Edge Connections

**Before:**
```rust
graph.add_edge(Edge {
    source_node: "producer".to_string(),
    source_port: "out".to_string(),
    target_node: "transform".to_string(),
    target_port: "in".to_string(),
})?;
```

**After:**
```rust
graph! {
    // ... nodes ...
    ; producer.out => transform.in
}
```

**Key Changes:**
- Connections follow a semicolon after node definitions
- Syntax: `source_node.source_port => target_node.target_port`
- All ports must be explicitly named (no defaults)
- Multiple connections separated by commas

### Step 4: Convert Graph I/O

#### Graph Inputs

**Before:**
```rust
graph.expose_input_port("producer", "in", "input")?;
// Then connect channel at runtime
graph.connect_input_channel("input", input_rx)?;
```

**After (without initial value):**
```rust
graph! {
    producer: producer_node,
    ; graph.input => producer.in
}
```

**After (with initial value):**
```rust
graph! {
    producer: producer_node,
    ; graph.config: 42i32 => producer.in
}
```

#### Graph Outputs

**Before:**
```rust
graph.expose_output_port("sink", "out", "output")?;
graph.connect_output_channel("output", output_tx)?;
```

**After:**
```rust
graph! {
    sink: sink_node,
    ; sink.out => graph.output
}
```

## Common Patterns

### Linear Pipeline

**Traditional:**
```rust
let mut graph = Graph::new("pipeline".to_string());
graph.add_node("source".to_string(), Box::new(source))?;
graph.add_node("transform".to_string(), Box::new(transform))?;
graph.add_node("sink".to_string(), Box::new(sink))?;
graph.add_edge(Edge {
    source_node: "source".to_string(),
    source_port: "out".to_string(),
    target_node: "transform".to_string(),
    target_port: "in".to_string(),
})?;
graph.add_edge(Edge {
    source_node: "transform".to_string(),
    source_port: "out".to_string(),
    target_node: "sink".to_string(),
    target_port: "in".to_string(),
})?;
```

**With Macro:**
```rust
let mut graph: Graph = graph! {
    source: source,
    transform: transform,
    sink: sink,
    ; source.out => transform.in,
      transform.out => sink.in
};
```

### Graph I/O with Values

**Traditional:**
```rust
graph.expose_input_port("node", "in", "config")?;
let (tx, rx) = mpsc::channel(1);
graph.connect_input_channel("config", rx)?;
tx.send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>).await?;
```

**With Macro:**
```rust
let mut graph: Graph = graph! {
    node: node,
    ; graph.config: 42i32 => node.in
};
// Value is automatically sent when graph is built
```

### Fan-In Pattern

**Traditional:**
```rust
graph.add_edge(Edge {
    source_node: "source1".to_string(),
    source_port: "out".to_string(),
    target_node: "merge".to_string(),
    target_port: "in1".to_string(),
})?;
graph.add_edge(Edge {
    source_node: "source2".to_string(),
    source_port: "out".to_string(),
    target_node: "merge".to_string(),
    target_port: "in2".to_string(),
})?;
```

**With Macro:**
```rust
let mut graph: Graph = graph! {
    source1: source1,
    source2: source2,
    merge: merge,
    ; source1.out => merge.in1,
      source2.out => merge.in2
};
```

**Note:** Fan-in (multiple sources → one target) is supported. Fan-out (one source → multiple targets) is **not supported**.

## Restrictions and Limitations

### 1. Fan-Out Not Supported

**Cannot do:**
```rust
// This will panic at build time
graph! {
    source: source,
    target1: target1,
    target2: target2,
    ; source.out => target1.in,
      source.out => target2.in  // ERROR: Fan-out not supported
};
```

**Workaround:** Use intermediate nodes or restructure your graph.

### 2. Reserved Node Name

**Cannot do:**
```rust
// This will cause a compile error
graph! {
    graph: some_node  // ERROR: "graph" is reserved
};
```

**Reason:** `graph` is reserved for graph I/O namespace (`graph.input_name`, `graph.output_name`).

### 3. Explicit Ports Required

**Cannot do:**
```rust
// No default ports or sequential shortcuts
graph! {
    source: source,
    sink: sink,
    ; source => sink  // ERROR: Ports must be explicit
};
```

**Must do:**
```rust
graph! {
    source: source,
    sink: sink,
    ; source.out => sink.in  // Ports explicitly named
};
```

### 4. Multiple Connections in Macro

Currently, the macro supports multiple connections, but they must be separated by commas:

```rust
graph! {
    a: node_a,
    b: node_b,
    c: node_c,
    ; a.out => b.in, b.out => c.in  // Multiple connections
};
```

## What Stays the Same

The following operations remain unchanged and work the same way:

- **Graph execution**: `Graph::execute(&mut graph).await?`
- **External channel connections**: `graph.connect_input_channel()` and `graph.connect_output_channel()`
- **Graph lifecycle**: `start()`, `pause()`, `resume()`, `stop()`, `wait_for_completion()`
- **Node implementations**: All existing nodes work without modification
- **Runtime behavior**: Graphs built with the macro behave identically to traditionally built graphs

## Migration Checklist

- [ ] Import `graph!` macro: `use streamweave::graph;`
- [ ] Convert node additions to macro syntax
- [ ] Convert edge connections to macro syntax
- [ ] Convert graph I/O to macro syntax
- [ ] Remove manual `Edge` construction
- [ ] Remove string conversions for node/port names
- [ ] Test graph execution (should work identically)
- [ ] Update any documentation/comments

## Troubleshooting

### Compile Error: "Node name 'graph' is reserved"

**Problem:** You're trying to name a node `graph`.

**Solution:** Use a different name. `graph` is reserved for graph I/O namespace.

### Runtime Panic: "Fan-out not supported"

**Problem:** You're trying to connect the same source port to multiple targets.

**Solution:** Restructure your graph or use intermediate nodes. Fan-out requires stream duplication which isn't supported yet.

### Compile Error: "no rules expected `.`"

**Problem:** Macro parsing issue, often due to incorrect syntax.

**Solution:** Ensure:
- Connections follow a semicolon after nodes
- Port names are explicitly specified
- Syntax is: `node.port => node.port`

## Examples

See the following examples for complete migration patterns:

- `examples/graph_macro_simple.rs` - Basic linear pipeline
- `examples/graph_macro_fan_patterns.rs` - Fan-in patterns
- `examples/graph_macro_io.rs` - Graph I/O patterns

## Further Reading

- [Graph API Guide](GRAPH.md) - Advanced graph patterns and routing
- [API Documentation](https://docs.rs/streamweave) - Full API reference
- [Macro Documentation](../src/graph_macros.rs) - Macro implementation details
