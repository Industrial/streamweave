use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::stream::SkipNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (count_tx, count_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/stream/skip_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("SkipNode", |id, _inputs, _outputs| {
      Box::new(SkipNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("count", count_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with SkipNode using graph! macro");

  // Send configuration (optional for SkipNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send count: skip first 3 items
  let skip_count = 3usize;
  count_tx
    .send(Arc::new(skip_count) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending skip count: {}", skip_count);

  // Send test data: 6 items, first 3 should be skipped
  println!("📥 Sending values to skip: 1, 2, 3, 4, 5, 6");
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with SkipNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(count_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result
      && let Ok(value) = item.downcast::<i32>()
    {
      output_items.push(*value);
      has_data = true;
      println!("  Output: {}", *value);
    }

    if let Ok(Some(item)) = error_result
      && let Ok(error_msg) = item.downcast::<String>()
    {
      let error = (**error_msg).to_string();
      println!("  Error: {}", error);
      error_count += 1;
      has_data = true;
    }

    if !has_data {
      break;
    }
  }

  println!("✓ Received {} items via output channel", output_items.len());
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive items 4, 5, 6 (skipped first 3)
  let expected_output = vec![4i32, 5i32, 6i32];
  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ SkipNode correctly skipped first {} items and forwarded the rest",
      skip_count
    );
  } else {
    println!(
      "⚠ SkipNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
