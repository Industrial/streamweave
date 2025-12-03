# Graphviz Image Generation Example

This example demonstrates how to generate pipeline DAG visualizations as image files (PNG, SVG, PDF, JPG) using Graphviz.

## Overview

This example:
1. Creates a multi-stage pipeline
2. Generates a DAG representation
3. Converts the DAG to DOT format
4. Uses Graphviz to generate images in multiple formats (PNG, SVG, PDF, JPG)
5. Validates Graphviz availability before attempting conversion

## Prerequisites

**Graphviz must be installed** for this example to work. The example will check for Graphviz availability and provide installation instructions if it's not found.

### Installation Instructions

#### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install graphviz
```

#### Linux (Fedora/RHEL/CentOS)
```bash
sudo dnf install graphviz
# or
sudo yum install graphviz
```

#### Linux (Arch)
```bash
sudo pacman -S graphviz
```

#### macOS
```bash
brew install graphviz
```

#### Windows
1. Download Graphviz from https://graphviz.org/download/
2. Run the installer
3. Add Graphviz `bin` directory to your PATH environment variable
   - Typically: `C:\Program Files\Graphviz\bin`
4. Restart your terminal/command prompt

#### Verify Installation
```bash
dot -V
```

This should display the Graphviz version if installed correctly.

## Running the Example

```bash
cargo run --example visualization_dot_image
```

## Expected Output

```
🎨 StreamWeave Graphviz Image Generation Example
=================================================

🔍 Checking Graphviz availability...
✅ Graphviz is available!

📊 Generating pipeline DAG...
✅ DAG generated successfully!
   Nodes: 4
   Edges: 3

📝 Saving DOT file...
✅ DOT file saved: pipeline_dot_image.dot

🖼️  Generating images in multiple formats...
   ✓ PNG: pipeline_dot_image.png
   ✓ SVG: pipeline_dot_image.svg
   ✓ PDF: pipeline_dot_image.pdf
   ✓ JPG: pipeline_dot_image.jpg

✅ Image generation completed!
   Successfully generated: 4/4 formats
```

## Generated Files

After running the example, you'll find:

### Image Files

- **`pipeline_dot_image.png`**: PNG image (best for presentations and web)
- **`pipeline_dot_image.svg`**: Scalable vector format (great for web, scalable)
- **`pipeline_dot_image.pdf`**: PDF format (best for documents and printing)
- **`pipeline_dot_image.jpg`**: JPEG format (smaller file size, good for sharing)

### DOT File

- **`pipeline_dot_image.dot`**: Source DOT file that can be used to regenerate images with custom options

## Format Comparison

| Format | Best For | Pros | Cons |
|--------|----------|------|------|
| PNG | Presentations, web | Lossless, supports transparency | Larger file size |
| SVG | Web, scaling | Vector format, scalable, small file size | Not all tools support |
| PDF | Documents, printing | High quality, widely supported | Larger file size |
| JPG | Sharing, storage | Small file size | Lossy compression |

## Custom Graphviz Options

You can use the generated DOT file with custom Graphviz options:

```bash
# Generate with custom layout
dot -Tpng -Grankdir=TB pipeline_dot_image.dot -o custom.png

# Generate with custom node colors
dot -Tsvg -Ncolor=blue pipeline_dot_image.dot -o custom.svg

# Generate high-resolution PNG
dot -Tpng -Gdpi=300 pipeline_dot_image.dot -o highres.png

# Generate with different layout engine
neato -Tpng pipeline_dot_image.dot -o neato.png
```

## Graphviz Layout Engines

Graphviz provides multiple layout engines:

- **`dot`**: Hierarchical layouts (default, best for DAGs)
- **`neato`**: Spring model layouts
- **`fdp`**: Force-directed layouts
- **`sfdp`**: Scalable force-directed layouts
- **`circo`**: Circular layouts
- **`twopi`**: Radial layouts

## Troubleshooting

### Graphviz Not Found

If you see:
```
❌ Graphviz is not installed or not in PATH
```

**Solutions:**
1. Install Graphviz using the instructions above
2. Verify installation: `dot -V`
3. Check PATH: Ensure Graphviz `bin` directory is in your PATH
4. Restart terminal after installation

### Format Generation Fails

If a specific format fails to generate:

**Common Issues:**
- **PDF**: May require additional libraries on some systems
- **JPG**: Some Graphviz versions may not support JPEG directly (use PNG instead)
- **SVG**: Usually works on all platforms

**Solutions:**
1. Check Graphviz version: `dot -V`
2. Test format manually: `dot -Tpng pipeline_dot_image.dot -o test.png`
3. Update Graphviz to latest version

### Permission Errors

If you get permission errors when writing files:

**Solutions:**
1. Check write permissions in current directory
2. Run from a directory where you have write access
3. Use absolute paths if needed

## Code Structure

- `main.rs`: Main entry point with Graphviz integration
- `pipeline.rs`: Pipeline component creation

## Key Functions

### `check_graphviz_available()`

Checks if Graphviz `dot` command is available on the system.

### `convert_dot_to_image()`

Converts a DOT string to an image file using Graphviz.

**Parameters:**
- `dot_content`: The DOT format string
- `output_path`: Path where the image should be saved
- `format`: Image format ("png", "svg", "pdf", "jpg")

### `generate_images()`

Generates images in multiple formats from a DAG.

**Returns:**
- Vector of tuples `(format, path, success)` indicating which formats were generated

## Advanced Usage

### Custom DAG Generation

Modify the pipeline in `pipeline.rs` to create different DAG structures:

```rust
// Change the producer
let producer = ArrayProducer::new([/* your data */]);

// Add more transformers
let transformer3 = MapTransformer::new(|x| x * 2);

// Customize component names
let transformer = transformer.with_name("custom_name".to_string());
```

### Batch Processing

Process multiple DAGs:

```rust
let dags = vec![dag1, dag2, dag3];
for (i, dag) in dags.iter().enumerate() {
    generate_images(dag, &format!("dag_{}", i), &formats);
}
```

## Integration with CI/CD

This example can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Install Graphviz
  run: sudo apt-get install -y graphviz

- name: Generate DAG images
  run: cargo run --example visualization_dot_image
```

## Related Examples

- `visualization_basic`: Basic console export with DOT and JSON
- `visualization_html`: Web visualization with HTML output
- `visualization_graph`: Graph API visualization

## Tips

1. **PNG for Presentations**: Use PNG format for slides and presentations
2. **SVG for Web**: Use SVG for web pages (scalable, small file size)
3. **PDF for Documents**: Use PDF when including in documents
4. **DOT for Customization**: Keep the DOT file for custom Graphviz rendering
5. **High Resolution**: Use `-Gdpi=300` for high-resolution images

## Graphviz Resources

- **Documentation**: https://graphviz.org/documentation/
- **Gallery**: https://graphviz.org/gallery/
- **Node Shapes**: https://graphviz.org/doc/info/shapes.html
- **Colors**: https://graphviz.org/doc/info/colors.html

