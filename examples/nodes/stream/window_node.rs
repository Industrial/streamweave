use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::stream::WindowNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (size_tx, size_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    window: WindowNode::new("window".to_string()),
    graph.configuration => window.configuration,
    graph.input => window.in,
    graph.size => window.size,
    window.out => graph.output,
    window.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("size", size_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with WindowNode using graph! macro");

  // Send configuration (optional for WindowNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send window size: 3 items per window
  let window_size = 3usize;
  size_tx
    .send(Arc::new(window_size) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending window size: {}", window_size);

  // Send 8 items: should produce 2 complete windows, partial window discarded
  println!("📥 Sending 8 values: 1, 2, 3, 4, 5, 6, 7, 8");
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32, 8i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ All test data sent to input channel");

  // Execute the graph
  println!("Executing graph with WindowNode...");
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
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result {
      if let Ok(window) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let window_values: Vec<i32> = window
          .iter()
          .filter_map(|arc| arc.clone().downcast::<i32>().ok().map(|v| *v))
          .collect();
        println!("  Output window: {:?}", window_values);
        output_items.push(window_values);
        has_data = true;
      }
    }

    if let Ok(Some(item)) = error_result {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = (**error_msg).to_string();
        println!("  Error: {}", error);
        error_count += 1;
        has_data = true;
      }
    }

    if !has_data {
      break;
    }
  }

  println!(
    "✓ Received {} windows via output channel",
    output_items.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 2 complete windows: [1,2,3] and [4,5,6], partial [7,8] discarded
  let expected_windows = vec![vec![1i32, 2i32, 3i32], vec![4i32, 5i32, 6i32]];

  if output_items == expected_windows && error_count == 0 {
    println!(
      "✓ WindowNode correctly collected items into windows of size {}: {:?}",
      window_size, output_items
    );
    println!("  (Partial window [7, 8] was correctly discarded)");
  } else {
    println!(
      "⚠ WindowNode behavior may be unexpected (windows: {:?}, expected: {:?}, errors: {})",
      output_items, expected_windows, error_count
    );
  }

  Ok(())
}
