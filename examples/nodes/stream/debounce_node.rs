use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::stream::DebounceNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (delay_tx, delay_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    debounce: DebounceNode::new("debounce".to_string()),
    graph.configuration => debounce.configuration,
    graph.input => debounce.in,
    graph.delay => debounce.delay,
    debounce.out => graph.output,
    debounce.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("delay", delay_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with DebounceNode using graph! macro");

  // Send configuration (optional for DebounceNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send debounce delay: 200ms
  let debounce_delay = 200u64;
  delay_tx
    .send(Arc::new(debounce_delay) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending debounce delay: {}ms", debounce_delay);

  // Send test data: rapid-fire items that should be debounced
  println!("📥 Sending rapid-fire values: A, B, C (with delays < debounce time)");
  let test_values = ["A", "B", "C"];

  for (i, value) in test_values.iter().enumerate() {
    input_tx
      .send(Arc::new(value.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    println!("  Sent: {} (item {})", value, i + 1);

    // Wait less than debounce delay between items
    if i < test_values.len() - 1 {
      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
  }

  println!("✓ All rapid-fire items sent, waiting for debounce...");

  // Execute the graph
  println!("Executing graph with DebounceNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(delay_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(1000), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(1000), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result
      && let Ok(value) = item.downcast::<String>()
    {
      output_items.push(value.clone());
      println!("  Output: {}", *value);
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

  println!("✓ Received {} items via output channel", output_items.len());
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive only the final item "C" after debounce delay
  // Items "A" and "B" should have been cancelled by the rapid succession
  let expected_output = vec![Arc::new("C".to_string())];
  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ DebounceNode correctly debounced rapid-fire items, emitting only final value: {:?}",
      output_items
    );
  } else {
    println!(
      "⚠ DebounceNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
