# Graph API Visualization Example

This example demonstrates how to visualize StreamWeave graphs (Graph API) by converting them to DAG representations and generating HTML visualizations. It showcases both fan-out and fan-in graph patterns.

## Overview

This example creates two different graph patterns:

1. **Fan-Out Pattern**: One producer feeds multiple transformers, which then feed separate consumers
2. **Fan-In Pattern**: Multiple producers feed into a single transformer, which then feeds a consumer

Both patterns are converted to DAG format and visualized as interactive HTML files.

## Running the Example

```bash
cargo run --example visualization_graph
```

## Graph Patterns

### Fan-Out Pattern

One producer splits its output to multiple transformers:

```
     Producer (source)
        |
    +---+---+
    |       |
Trans1  Trans2
(double) (triple)
    |       |
Cons1   Cons2
(sink1) (sink2)
```

**Structure:**
- 1 Producer: Generates numbers 1-10
- 2 Transformers: One doubles, one triples
- 2 Consumers: Collect results separately

### Fan-In Pattern

Multiple producers feed into one transformer:

```
Prod1  Prod2
(source1) (source2)
   |     |
   +--+--+
      |
   Transform
   (process)
      |
   Consumer
   (sink)
```

**Structure:**
- 2 Producers: Generate different number ranges
- 1 Transformer: Processes the data
- 1 Consumer: Collects results

**Note:** For a true fan-in with merging, you would use a `MergeTransformer` or similar. This example demonstrates the structural pattern.

## Generated Files

After running the example, you'll find:

### HTML Visualizations

- **`graph_fan_out.html`**: Interactive visualization of the fan-out pattern
- **`graph_fan_in.html`**: Interactive visualization of the fan-in pattern

### JSON Data

- **`graph_fan_out.json`**: DAG data for fan-out pattern
- **`graph_fan_in.json`**: DAG data for fan-in pattern

### DOT Format

- **`graph_fan_out.dot`**: Graphviz DOT format for fan-out
- **`graph_fan_in.dot`**: Graphviz DOT format for fan-in

## Graph to DAG Conversion

The example includes a `graph_to_dag()` function that:

1. Extracts all nodes from the Graph
2. Converts node kinds (Producer, Transformer, Consumer) to DAG node kinds
3. Extracts connection information to create DAG edges
4. Creates metadata for each node

This conversion enables visualization of Graph API structures using the same DAG visualization tools used for Pipeline API.

## Code Structure

- `main.rs`: Main entry point with graph creation, DAG conversion, and visualization generation

## Key Functions

### `graph_to_dag()`

Converts a `Graph` to a `PipelineDag` for visualization:

```rust
fn graph_to_dag(graph: &Graph) -> PipelineDag {
    // Extracts nodes and connections
    // Creates DAG representation
}
```

### `create_fan_out_graph()`

Creates a fan-out graph pattern with one producer feeding multiple transformers.

### `create_fan_in_graph()`

Creates a fan-in graph pattern with multiple producers feeding one transformer.

## Use Cases

This example is useful for:

- **Understanding Graph API Structure**: Visualize how graphs are constructed
- **Pattern Recognition**: See fan-out and fan-in patterns in action
- **Debugging**: Visualize graph topology to understand data flow
- **Documentation**: Generate diagrams for documentation

## ASCII Diagrams

The example includes ASCII diagrams showing the graph structure:

```
Fan-Out Pattern:
  source → double → sink1
       └→ triple → sink2

Fan-In Pattern:
  source1 ┐
         ├→ process → sink
  source2 ┘
```

## Interactive Visualization Features

The generated HTML files include:

- Interactive graph rendering
- Zoom and pan controls
- Node detail view on click
- Export functionality (JSON, DOT)
- Reset zoom and fit-to-screen controls

## Related Examples

- `visualization_basic`: Basic pipeline visualization
- `visualization_html`: Enhanced web visualization with auto-open
- `http_graph_server`: Real-world graph API usage

## Tips

1. **True Fan-In**: For actual merging of multiple streams, use `MergeTransformer` or similar transformers that support multiple inputs
2. **Complex Patterns**: Combine fan-out and fan-in for more complex topologies
3. **Custom Visualization**: Use the JSON format with custom visualization tools
4. **Graphviz**: Generate static images from DOT files:
   ```bash
   dot -Tpng graph_fan_out.dot -o graph_fan_out.png
   ```

