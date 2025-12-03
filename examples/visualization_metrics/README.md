# Real-Time Metrics Visualization Example

This example demonstrates how to collect and visualize real-time metrics from a StreamWeave pipeline, including throughput, latency, and error rates.

## Overview

This example:
1. Creates a multi-stage pipeline
2. Collects real-time metrics during pipeline execution
3. Aggregates metrics (throughput, latency percentiles, error counts)
4. Generates HTML visualization with embedded metrics and charts
5. Exports metrics snapshots in JSON format

## Running the Example

```bash
cargo run --example visualization_metrics
```

## Expected Output

```
🎨 StreamWeave Real-Time Metrics Visualization Example
======================================================

📊 Generating pipeline DAG...
✅ DAG generated successfully!
   Nodes: 4
   Edges: 3

📈 Initializing metrics collection...
✅ Metrics collection initialized!

🔄 Simulating pipeline execution (collecting metrics)...
✅ Pipeline execution simulated!
   Items processed: 50
   Metrics collected for 4 nodes

📊 Metrics Summary:
   producer:
     Throughput: 10.00 items/sec
     Total Items: 50
     Errors: 0
     Avg Latency: 1.00 ms
     P95 Latency: 1.00 ms
   ...
```

## Generated Files

After running the example, you'll find:

### `pipeline_metrics_visualization.html`

An interactive HTML file that includes:
- DAG visualization of the pipeline structure
- Embedded metrics data for each node
- Metrics display with:
  - Throughput (items per second)
  - Total items processed
  - Error counts
  - Average latency
  - Latency percentiles (P50, P95, P99)

### `pipeline_metrics.json`

JSON file containing metrics snapshots for all nodes:

```json
[
  {
    "node_id": "producer",
    "throughput": 10.0,
    "total_items": 50,
    "error_count": 0,
    "avg_latency_ms": 1.0,
    "p50_latency_ms": 1.0,
    "p95_latency_ms": 1.0,
    "p99_latency_ms": 1.0
  },
  ...
]
```

### Additional Files

- `pipeline_metrics_dag.json`: DAG structure in JSON format
- `pipeline_metrics_dag.dot`: DAG structure in DOT format

## Metrics Collected

### Throughput

- **Definition**: Items processed per second
- **Calculation**: Based on items processed in a 5-second rolling window
- **Use Case**: Identify processing bottlenecks and overall pipeline performance

### Latency

- **Average Latency**: Mean processing time per item
- **P50 Latency**: Median processing time (50th percentile)
- **P95 Latency**: 95th percentile processing time
- **P99 Latency**: 99th percentile processing time
- **Use Case**: Understand processing time distribution and identify outliers

### Error Count

- **Definition**: Total number of errors encountered at each node
- **Use Case**: Monitor error rates and identify problematic components

### Total Items

- **Definition**: Total number of items processed by the node
- **Use Case**: Track overall processing volume

## Metrics Collection Process

The example demonstrates metrics collection through:

1. **Node Metrics Creation**: Each node gets its own `NodeMetrics` instance
2. **Event Recording**: Metrics are recorded as items flow through the pipeline:
   - `record_item_processed()`: Tracks throughput
   - `record_latency()`: Tracks processing time
   - `record_error()`: Tracks errors
3. **Snapshot Generation**: Final metrics are captured as serializable snapshots
4. **Visualization**: Metrics are embedded in HTML for interactive viewing

## HTML Visualization Features

The generated HTML includes:

- **DAG Visualization**: Interactive graph showing pipeline structure
- **Metrics Display**: Each node's metrics displayed in a card format
- **Real-Time Data**: Metrics data embedded in the HTML (no server required)
- **Responsive Layout**: Metrics displayed in a grid layout

## Code Structure

- `main.rs`: Main entry point with metrics collection and visualization
- `pipeline.rs`: Pipeline component creation

## Key Functions

### `simulate_pipeline_execution()`

Simulates pipeline execution and collects metrics for each node.

**Parameters:**
- `metrics`: The `PipelineMetrics` instance to collect into
- `num_items`: Number of items to simulate processing

**Returns:**
- Vector of `NodeMetricsSnapshot` for all nodes

### `enhance_html_with_metrics()`

Enhances the standard HTML output with embedded metrics visualization.

**Parameters:**
- `html`: Base HTML from `generate_standalone_html()`
- `metrics_snapshots`: Vector of metrics snapshots to embed

**Returns:**
- Enhanced HTML string with metrics visualization

## Integration with Real Pipelines

To integrate metrics collection with actual pipelines:

```rust
use streamweave::visualization::realtime::PipelineMetrics;

// Create metrics collection
let pipeline_metrics = PipelineMetrics::new();

// Get metrics for a node
let mut node_metrics = pipeline_metrics
    .get_or_create_node("my_node".to_string())
    .await;

// Record events during pipeline execution
node_metrics.record_item_processed().await;
node_metrics.record_latency(Duration::from_millis(10)).await;

// Get final snapshot
let snapshot = node_metrics.snapshot();
```

## Metrics Analysis

### Identifying Bottlenecks

Metrics can help identify bottlenecks:

- **Low Throughput**: Node processing fewer items than expected
- **High Latency**: Node taking longer to process items
- **High Error Rate**: Node encountering frequent errors

### Performance Optimization

Use metrics to optimize pipelines:

- **Throughput Analysis**: Identify nodes limiting overall throughput
- **Latency Analysis**: Find nodes with high processing times
- **Error Analysis**: Identify nodes with high error rates

## Programmatic Access

The JSON export can be used for programmatic analysis:

```rust
use serde_json;
use streamweave::visualization::realtime::NodeMetricsSnapshot;

let json = std::fs::read_to_string("pipeline_metrics.json")?;
let snapshots: Vec<NodeMetricsSnapshot> = serde_json::from_str(&json)?;

for snapshot in snapshots {
    println!("Node: {}", snapshot.node_id);
    println!("  Throughput: {} items/sec", snapshot.throughput);
    println!("  P95 Latency: {} ms", snapshot.p95_latency_ms);
}
```

## Advanced Usage

### Custom Metrics Collection

Extend metrics collection for custom metrics:

```rust
// Add custom fields to NodeMetricsSnapshot
// Track additional metrics like memory usage, CPU, etc.
```

### Real-Time Updates

For real-time metrics updates, periodically snapshot metrics:

```rust
loop {
    let snapshots = pipeline_metrics.snapshot_all().await;
    // Send snapshots to visualization frontend
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### Metrics Export

Export metrics to monitoring systems:

- **Prometheus**: Use Prometheus exporter
- **OpenTelemetry**: Use OpenTelemetry integration
- **Custom**: Export JSON to your monitoring system

## Related Examples

- `visualization_basic`: Basic console export
- `visualization_html`: Web visualization without metrics
- `visualization_graph`: Graph API visualization
- `metrics_monitoring`: General metrics monitoring example

## Tips

1. **Sampling Rate**: Adjust metrics collection frequency based on pipeline throughput
2. **Window Size**: Modify the throughput calculation window for different time scales
3. **Percentile Calculation**: Adjust sample size for more accurate percentiles
4. **Error Tracking**: Add error type classification for better error analysis
5. **Historical Data**: Store metrics snapshots over time for trend analysis

## Performance Considerations

- Metrics collection has minimal overhead
- Use async operations for metrics updates
- Consider sampling for high-throughput pipelines
- Store metrics snapshots periodically to avoid memory growth

