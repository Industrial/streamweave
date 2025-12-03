# Enhanced Web Visualization Example with Browser Auto-Open

This example demonstrates how to generate a standalone HTML file for visualizing pipeline DAGs in a web browser, with automatic browser opening functionality.

## Overview

This example creates a complex multi-stage pipeline with:
- **Producer**: `ArrayProducer` that generates numbers from 1 to 20
- **Transformer 1**: `MapTransformer` that squares each number
- **Transformer 2**: `FilterTransformer` that filters to keep only numbers greater than 50
- **Consumer**: `VecConsumer` that collects results into a vector

The example then:
1. Generates a DAG representation of the pipeline
2. Creates a standalone HTML file with embedded DAG data
3. Automatically opens the HTML file in your default browser (cross-platform)
4. Exports additional formats (JSON, DOT)

## Running the Example

```bash
cargo run --example visualization_html
```

The example will automatically attempt to open the generated HTML file in your default web browser.

## Features

### Browser Auto-Open

The example includes cross-platform browser opening functionality:

- **Linux**: Uses `xdg-open` command
- **macOS**: Uses `open` command
- **Windows**: Uses `cmd /c start` command

If automatic opening fails, the example will display instructions for manual opening.

### Interactive Visualization

The generated HTML file includes:
- Interactive DAG visualization with zoom and pan
- Click on nodes to see component details
- Export buttons for JSON and DOT formats
- Reset zoom and fit-to-screen controls

## Expected Output

```
🎨 StreamWeave Enhanced Web Visualization Example
===================================================

📊 Generating pipeline DAG...
✅ DAG generated successfully!
   Nodes: 4
   Edges: 3

🌐 Generating standalone HTML visualization...
✅ HTML file generated: pipeline_html_visualization.html

🌍 Attempting to open browser automatically...
✅ Browser opened successfully!

📝 Exporting additional formats...
   ✓ JSON: pipeline_html_dag.json
   ✓ DOT: pipeline_html_dag.dot
```

## Generated Files

After running the example, you'll find three files in the current directory:

### `pipeline_html_visualization.html`

A standalone HTML file with embedded DAG data that can be opened directly in any browser. No web server required!

**Features:**
- Interactive graph visualization
- Zoom and pan controls
- Node detail view on click
- Export functionality

### `pipeline_html_dag.json`

A JSON file containing the complete DAG structure for programmatic access.

### `pipeline_html_dag.dot`

A Graphviz DOT format file for generating static images:

```bash
# Generate PNG
dot -Tpng pipeline_html_dag.dot -o pipeline_html_dag.png

# Generate SVG
dot -Tsvg pipeline_html_dag.dot -o pipeline_html_dag.svg
```

## Pipeline Structure

The example pipeline has the following structure:

```
ArrayProducer (1-20)
    ↓ [i32]
MapTransformer (squares: x * x)
    ↓ [i32]
FilterTransformer (filters: x > 50)
    ↓ [i32]
VecConsumer (collects)
```

**Example Flow:**
- Input: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, ...]
- After MapTransformer: [1, 4, 9, 16, 25, 36, 49, 64, 81, 100, ...]
- After FilterTransformer: [64, 81, 100, 121, 144, 169, 196, 225, 256, 289, 324, 361, 400]

## Troubleshooting

### Browser Doesn't Open Automatically

**Linux:**
- Ensure `xdg-open` is installed:
  ```bash
  # Ubuntu/Debian
  sudo apt-get install xdg-utils
  
  # Fedora
  sudo dnf install xdg-utils
  
  # Arch
  sudo pacman -S xdg-utils
  ```

**macOS:**
- The `open` command should be available by default
- If it fails, check your system permissions

**Windows:**
- The `start` command should work automatically
- If it fails, try opening the HTML file manually

### Manual Browser Opening

If automatic opening fails, you can manually open the HTML file:

```bash
# Linux
xdg-open pipeline_html_visualization.html

# macOS
open pipeline_html_visualization.html

# Windows
start pipeline_html_visualization.html
```

Or simply double-click the HTML file in your file manager.

### HTML File Not Displaying Correctly

- Ensure you're opening the file directly (not through a web server)
- Check browser console for JavaScript errors
- Try a different browser if issues persist
- Verify the file was generated completely (check file size)

### Browser Security Warnings

Some browsers may show security warnings when opening local HTML files. This is normal and you can safely allow the file to load.

## Platform-Specific Notes

### Linux

- Requires `xdg-open` (usually pre-installed on desktop environments)
- Works with GNOME, KDE, XFCE, and other desktop environments
- May require desktop environment to be running

### macOS

- Uses the built-in `open` command
- Opens in your default browser (Safari, Chrome, Firefox, etc.)
- Works from Terminal and other command-line environments

### Windows

- Uses `cmd /c start` command
- Opens in your default browser
- Works from Command Prompt, PowerShell, and Git Bash

## Code Structure

- `main.rs`: Main entry point with browser auto-open functionality
- `pipeline.rs`: Complex pipeline component creation

## Browser Auto-Open Implementation

The browser opening is implemented using platform-specific commands:

```rust
#[cfg(target_os = "windows")]
Command::new("cmd").args(["/C", "start", "", &path_str]).spawn()?;

#[cfg(target_os = "macos")]
Command::new("open").arg(&path_str).spawn()?;

#[cfg(target_os = "linux")]
Command::new("xdg-open").arg(&path_str).spawn()?;
```

This approach:
- ✅ No external dependencies required
- ✅ Works on all major platforms
- ✅ Uses standard system commands
- ✅ Gracefully handles failures

## Related Examples

- `visualization_basic`: Basic console export with DOT and JSON
- `visualization_web`: Web visualization without auto-open
- `visualization_graph`: Graph API visualization examples

## Advanced Usage

### Customizing the Visualization

You can modify the pipeline in `pipeline.rs` to create different DAG structures:

```rust
// Change the producer
let producer = ArrayProducer::new([/* your data */]);

// Add more transformers
let transformer3 = MapTransformer::new(|x| x * 2);

// Customize component names
let transformer = transformer.with_name("custom_name".to_string());
```

### Programmatic Access

The JSON export can be used for programmatic analysis:

```rust
use serde_json;
let dag: PipelineDag = serde_json::from_str(&json_string)?;
// Process DAG programmatically
```

## Tips

1. **Multiple Visualizations**: Run the example multiple times to generate different visualizations
2. **Sharing**: The HTML file is completely standalone - share it with others easily
3. **Integration**: Use the JSON format to integrate with other tools
4. **Documentation**: Include generated HTML files in your project documentation

