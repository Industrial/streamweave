use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::stream::BufferNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (size_tx, size_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/stream/buffer_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("BufferNode", |id, _inputs, _outputs| {
      Box::new(BufferNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("size", size_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with BufferNode using graph! macro");

  // Send configuration (optional for BufferNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send buffer size: 3 items per batch
  let buffer_size = 3usize;
  size_tx
    .send(Arc::new(buffer_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending buffer size: {}", buffer_size);

  // Send test data: 8 items, should produce 2 full batches (3+3) and discard the last 2
  println!("📥 Sending values to buffer: 1, 2, 3, 4, 5, 6, 7, 8");
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32, 8i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with BufferNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(size_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_batches = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result
      && let Ok(batch) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    {
      output_batches.push(batch.clone());
      println!(
        "  Output batch: {:?}",
        batch
          .iter()
          .map(|item| { item.clone().downcast::<i32>().map(|v| *v).unwrap_or(0) })
          .collect::<Vec<i32>>()
      );
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
    "✓ Received {} batches via output channel",
    output_batches.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 2 batches of 3 items each, partial batch discarded
  let expected_batches = [vec![1i32, 2i32, 3i32], vec![4i32, 5i32, 6i32]];

  if output_batches.len() == expected_batches.len() && error_count == 0 {
    let mut all_correct = true;
    for (i, batch) in output_batches.iter().enumerate() {
      let expected_batch = &expected_batches[i];
      if batch.len() != expected_batch.len() {
        all_correct = false;
        break;
      }
      for (j, item) in batch.iter().enumerate() {
        if let Ok(value) = item.clone().downcast::<i32>() {
          if *value != expected_batch[j] {
            all_correct = false;
            break;
          }
        } else {
          all_correct = false;
          break;
        }
      }
      if !all_correct {
        break;
      }
    }

    if all_correct {
      println!(
        "✓ BufferNode correctly buffered items into batches of {}",
        buffer_size
      );
    } else {
      println!("⚠ Batch contents do not match expected values");
    }
  } else {
    println!(
      "⚠ Unexpected behavior (batches: {}, expected: {}, errors: {})",
      output_batches.len(),
      expected_batches.len(),
      error_count
    );
  }

  Ok(())
}
