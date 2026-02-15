use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::stream::MergeNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in0_tx, in0_rx) = mpsc::channel(10);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(20);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/stream/merge_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("MergeNode", |id, inputs, _outputs| {
      Box::new(MergeNode::new(id, inputs.len()))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input0", in0_rx)?;
  graph.connect_input_channel("input1", in1_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with MergeNode using graph! macro");

  // Send configuration (optional for MergeNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send data to two input streams
  println!("📥 Sending values to stream 0: 1, 3, 5");
  let stream0_values = vec![1i32, 3i32, 5i32];
  for value in stream0_values {
    in0_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Stagger sends
  }

  println!("📥 Sending values to stream 1: 2, 4, 6");
  let stream1_values = vec![2i32, 4i32, 6i32];
  for value in stream1_values {
    in1_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await; // Different timing
  }

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with MergeNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in0_tx);
  drop(in1_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result
      && let Ok(value) = item.downcast::<i32>()
    {
      let num = *value;
      output_items.push(num);
      println!("  Received merged value: {}", num);
      has_data = true;
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

  println!(
    "✓ Received {} merged items via output channel",
    output_items.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Sort the output for deterministic checking (since merge order is non-deterministic)
  output_items.sort();

  // Verify behavior: should receive all 6 items (1,2,3,4,5,6) in some order, and no errors
  if output_items.len() == 6 && output_items == vec![1, 2, 3, 4, 5, 6] && error_count == 0 {
    println!("✓ MergeNode correctly merged both input streams");
    println!("  Merged items: {:?}", output_items);
  } else {
    println!(
      "⚠ MergeNode behavior may be unexpected (received: {:?}, expected: [1,2,3,4,5,6], errors: {})",
      output_items, error_count
    );
  }

  Ok(())
}
