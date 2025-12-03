# Debug Mode Visualization Example

This example demonstrates how to create debug-mode visualizations with breakpoint support for StreamWeave pipelines.

## Overview

This example:
1. Sets up breakpoints for debugging pipeline execution
2. Creates debug visualization with enhanced DAG metadata
3. Generates HTML with interactive debug UI
4. Exports breakpoint configuration in JSON format

## Running the Example

```bash
cargo run --example visualization_debug
```

## Expected Output

```
🎨 StreamWeave Debug Mode Visualization Example
===============================================

🐛 Initializing debugger...
✅ Debugger initialized!

📍 Setting up breakpoints...
✅ Breakpoints configured!
   Breakpoint 1: transformer1 (always)
   Breakpoint 2: transformer2 (condition: item > 10)

📊 Generating DAG with debug metadata...
✅ DAG generated successfully!
   Nodes: 4
   Edges: 3
   Debug metadata: Enabled

🌐 Generating HTML visualization with debug UI...
✅ Enhanced HTML file generated: pipeline_debug_visualization.html
```

## Generated Files

After running the example, you'll find:

### `pipeline_debug_visualization.html`

An interactive HTML file that includes:
- DAG visualization of the pipeline structure
- Debug UI panel with:
  - Current debug state display
  - Breakpoints list with status
  - Debug controls (Resume, Step, Pause)
- Enhanced metadata showing debug-enabled nodes

### `pipeline_debug_breakpoints.json`

JSON file containing breakpoint configuration:

```json
[
  {
    "node_id": "transformer1",
    "condition": null,
    "enabled": true
  },
  {
    "node_id": "transformer2",
    "condition": "item > 10",
    "enabled": true
  }
]
```

### `pipeline_debug_state.json`

JSON file containing current debug state:

```json
"Running"
```

### Additional Files

- `pipeline_debug_dag.json`: DAG structure with debug metadata
- `pipeline_debug_dag.dot`: DAG structure in DOT format

## Breakpoint Types

### Simple Breakpoint

Pauses execution whenever the node is reached:

```rust
let breakpoint = Breakpoint::at_node("transformer1".to_string());
```

### Conditional Breakpoint

Pauses execution only when a condition is met:

```rust
let breakpoint = Breakpoint::with_condition(
    "transformer2".to_string(),
    "item > 10".to_string(),
);
```

## Debug States

The debugger can be in one of several states:

- **Running**: Pipeline executing normally
- **Paused**: Execution paused at a breakpoint
- **Stepping**: Step-through mode enabled
- **Completed**: Pipeline execution finished

## Debug Workflow

### 1. Set Breakpoints

Configure breakpoints at nodes where you want to pause:

```rust
let mut debugger = Debugger::new();
debugger.add_breakpoint(Breakpoint::at_node("transformer1".to_string())).await;
```

### 2. Run Pipeline

Execute the pipeline - it will pause at breakpoints:

```rust
// Pipeline execution pauses when breakpoint is hit
// Debug state changes to Paused
```

### 3. Inspect Data

When paused, inspect data snapshots:

```rust
let snapshots = debugger.get_snapshots().await;
for snapshot in snapshots {
    println!("Node: {}, Data: {}", snapshot.node_id, snapshot.data);
}
```

### 4. Control Execution

Use debug controls to manage execution:

- **Resume**: Continue execution until next breakpoint
- **Step**: Execute one item and pause
- **Pause**: Pause execution immediately

### 5. Export Configuration

Save breakpoint configuration for reuse:

```rust
let breakpoints_json = serde_json::to_string_pretty(&breakpoints)?;
std::fs::write("breakpoints.json", breakpoints_json)?;
```

## HTML Debug UI Features

The generated HTML includes:

- **Debug State Display**: Shows current debug state (Running, Paused, Stepping)
- **Breakpoints List**: Lists all configured breakpoints with status
- **Debug Controls**: Interactive buttons for Resume, Step, and Pause
- **Enhanced DAG**: Nodes with breakpoints are visually marked

## Code Structure

- `main.rs`: Main entry point with debugger setup and visualization
- `pipeline.rs`: Pipeline component creation

## Key Functions

### `create_dag_with_debug_metadata()`

Creates a DAG with enhanced metadata for debug visualization.

**Features:**
- Adds `debug_enabled` flag to node metadata
- Marks nodes with breakpoints
- Includes debug information in custom fields

### `enhance_html_with_debug()`

Enhances the standard HTML output with debug UI.

**Features:**
- Embeds breakpoint configuration
- Adds debug state display
- Includes interactive debug controls

## Breakpoint Configuration

### Loading Breakpoints from JSON

```rust
let json = std::fs::read_to_string("breakpoints.json")?;
let breakpoints: Vec<Breakpoint> = serde_json::from_str(&json)?;

for breakpoint in breakpoints {
    debugger.add_breakpoint(breakpoint).await;
}
```

### Saving Breakpoints to JSON

```rust
let breakpoints = debugger.breakpoints.read().await.clone();
let json = serde_json::to_string_pretty(&breakpoints)?;
std::fs::write("breakpoints.json", json)?;
```

## Debug Metadata in DAG

Nodes in the DAG include debug metadata:

- `debug_enabled`: Indicates debug mode is active
- `has_breakpoint`: Indicates a breakpoint is set at this node
- Custom metadata can be added for additional debug information

## Use Cases

### Development

- **Step-through Debugging**: Step through pipeline execution item by item
- **Data Inspection**: Inspect data at specific points in the pipeline
- **Error Investigation**: Set breakpoints near error-prone nodes

### Testing

- **Conditional Breakpoints**: Pause only when specific conditions are met
- **Test Scenarios**: Create test scenarios with specific breakpoint configurations
- **Regression Testing**: Use saved breakpoint configurations for regression tests

### Documentation

- **Visual Debug Guides**: Generate HTML visualizations showing debug points
- **Breakpoint Documentation**: Export breakpoint configurations for documentation
- **Debug Workflows**: Document debugging workflows using saved configurations

## Advanced Usage

### Conditional Breakpoints

Use conditional breakpoints to pause only under specific conditions:

```rust
// Pause only when item value is greater than 100
let breakpoint = Breakpoint::with_condition(
    "transformer1".to_string(),
    "item.value > 100".to_string(),
);
```

### Multiple Breakpoints

Set breakpoints at multiple nodes:

```rust
debugger.add_breakpoint(Breakpoint::at_node("producer".to_string())).await;
debugger.add_breakpoint(Breakpoint::at_node("transformer1".to_string())).await;
debugger.add_breakpoint(Breakpoint::at_node("consumer".to_string())).await;
```

### Breakpoint Management

Enable/disable breakpoints dynamically:

```rust
let mut breakpoint = Breakpoint::at_node("transformer1".to_string());
breakpoint.disable(); // Temporarily disable
// ... later ...
breakpoint.enable(); // Re-enable
```

## Integration with Real Pipelines

To integrate debug mode with actual pipelines:

```rust
use streamweave::visualization::debug::{Debugger, Breakpoint};

// Create debugger
let mut debugger = Debugger::new();

// Set breakpoints
debugger.add_breakpoint(Breakpoint::at_node("my_node".to_string())).await;

// Check if should pause during pipeline execution
if debugger.should_pause("my_node".to_string(), Some(data_serialized)).await {
    // Pause execution, inspect data, etc.
    let snapshots = debugger.get_snapshots().await;
    // ... inspect snapshots ...
    
    // Resume when ready
    debugger.resume().await;
}
```

## Related Examples

- `visualization_basic`: Basic console export
- `visualization_html`: Web visualization without debug
- `visualization_metrics`: Metrics visualization
- `visualization_graph`: Graph API visualization

## Tips

1. **Breakpoint Placement**: Set breakpoints at key transformation points
2. **Conditional Breakpoints**: Use conditions to focus on specific data patterns
3. **Export Configurations**: Save breakpoint configurations for reuse
4. **Debug State**: Monitor debug state to understand execution flow
5. **Data Snapshots**: Use snapshots to inspect data without modifying it

## Debug Workflow Best Practices

1. **Start Simple**: Begin with simple breakpoints, add conditions later
2. **Incremental Debugging**: Add breakpoints one at a time
3. **Document Breakpoints**: Use meaningful node names and condition descriptions
4. **Export Configurations**: Save working breakpoint configurations
5. **Clean Up**: Remove breakpoints when no longer needed

## Troubleshooting

### Breakpoints Not Triggering

- Verify node IDs match exactly
- Check that breakpoints are enabled
- Ensure pipeline execution reaches the node

### Debug State Not Updating

- Verify debugger is properly initialized
- Check that debug state is being read correctly
- Ensure async operations complete

### HTML Debug UI Not Showing

- Verify HTML file was generated correctly
- Check browser console for JavaScript errors
- Ensure debug configuration is embedded in HTML

