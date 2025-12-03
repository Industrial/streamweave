# Enhanced Console Export Visualization Example

This example demonstrates how to generate and export pipeline DAG (Directed Acyclic Graph) representations in multiple formats for visualization and debugging purposes.

## Overview

This example creates a multi-stage pipeline with:
- **Producer**: `ArrayProducer` that generates numbers from 1 to 10
- **Transformer 1**: `MapTransformer` that doubles each number
- **Transformer 2**: `FilterTransformer` that filters to keep only even numbers
- **Consumer**: `VecConsumer` that collects results into a vector

The example then:
1. Generates a DAG representation of the pipeline
2. Exports the DAG to DOT format (for Graphviz visualization)
3. Exports the DAG to JSON format (for programmatic access)
4. Saves both formats to files

## Running the Example

```bash
cargo run --example visualization_basic
```

## Expected Output

The example will:
1. Display the generated DAG structure (nodes and edges count)
2. Print the DOT format representation to the console
3. Print the JSON format representation to the console
4. Save `pipeline_basic.dot` and `pipeline_basic.json` files in the current directory

### Sample Output

```
🎨 StreamWeave Enhanced Console Export Visualization Example
============================================================

📊 Generating pipeline DAG...
✅ DAG generated successfully!
   Nodes: 4
   Edges: 3

📝 Exporting to DOT format (Graphviz)...
digraph PipelineDag {
  ...
}

💾 DOT format saved to: pipeline_basic.dot

📄 Exporting to JSON format...
{
  "nodes": [...],
  "edges": [...]
}

💾 JSON format saved to: pipeline_basic.json
```

## Generated Files

After running the example, you'll find two files in the current directory:

### `pipeline_basic.dot`

A Graphviz DOT format file that can be used to generate visual diagrams:

```bash
# Generate PNG image
dot -Tpng pipeline_basic.dot -o pipeline_basic.png

# Generate SVG image
dot -Tsvg pipeline_basic.dot -o pipeline_basic.svg

# Generate PDF
dot -Tpdf pipeline_basic.dot -o pipeline_basic.pdf
```

### `pipeline_basic.json`

A JSON file containing the complete DAG structure with:
- Node information (type, name, metadata)
- Edge information (connections between nodes)
- Component metadata (error strategies, input/output types)

This format is useful for:
- Web-based visualization tools
- Programmatic analysis
- Integration with other tools

## Pipeline Structure

The example pipeline has the following structure:

```
ArrayProducer (1-10)
    ↓ [i32]
MapTransformer (doubles)
    ↓ [i32]
FilterTransformer (even filter)
    ↓ [i32]
VecConsumer (collects)
```

## DAG Components

### Nodes

Each node in the DAG represents a pipeline component and includes:
- **ID**: Unique identifier
- **Kind**: Producer, Transformer, or Consumer
- **Metadata**: Component type, name, input/output types, error strategy

### Edges

Each edge represents data flow and includes:
- **From**: Source node ID
- **To**: Target node ID
- **Label**: Data type flowing through the edge

## Use Cases

This example is useful for:
- **Debugging**: Visualizing pipeline structure to understand data flow
- **Documentation**: Generating diagrams for documentation
- **Analysis**: Programmatically analyzing pipeline structure
- **Integration**: Using DAG data with external visualization tools

## Tips

1. **Graphviz Installation**: To generate images from DOT files, install Graphviz:
   ```bash
   # macOS
   brew install graphviz
   
   # Ubuntu/Debian
   sudo apt-get install graphviz
   
   # Windows
   # Download from https://graphviz.org/download/
   ```

2. **JSON Processing**: The JSON format can be easily parsed and processed:
   ```rust
   use serde_json;
   let dag: PipelineDag = serde_json::from_str(&json_string)?;
   ```

3. **Custom Visualization**: Use the JSON format with web visualization libraries like:
   - D3.js
   - vis.js
   - Cytoscape.js

## Code Structure

- `main.rs`: Main entry point that creates the pipeline, generates DAG, and exports formats
- `pipeline.rs`: Pipeline component creation and configuration

## Related Examples

- `visualization`: Basic visualization example with single transformer
- `visualization_web`: Web-based visualization with HTML output
- `visualization_graph`: Graph API visualization examples

