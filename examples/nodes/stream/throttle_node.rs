use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::stream::ThrottleNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (period_tx, period_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("throttle_example".to_string());
  graph.add_node(
    "throttle".to_string(),
    Box::new(ThrottleNode::new("throttle".to_string())),
  )?;
  graph.expose_input_port("throttle", "configuration", "configuration")?;
  graph.expose_input_port("throttle", "in", "input")?;
  graph.expose_input_port("throttle", "period", "period")?;
  graph.expose_output_port("throttle", "out", "output")?;
  graph.expose_output_port("throttle", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("period", period_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ThrottleNode using Graph API");

  // Send configuration (optional for ThrottleNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send throttle period: 300ms
  let throttle_period = 300u64;
  period_tx
    .send(Arc::new(throttle_period) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending throttle period: {}ms", throttle_period);

  // Send multiple values rapidly: 1, 2, 3, 4, 5
  // Only the first should be emitted immediately, others should be throttled
  println!("📥 Sending rapid values: 1, 2, 3, 4, 5");
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ All test data sent to input channel");

  // Execute the graph
  println!("Executing graph with ThrottleNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(period_tx);

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

    if let Ok(Some(item)) = output_result {
      if let Ok(value) = item.downcast::<i32>() {
        output_items.push(*value);
        println!("  Output: {}", *value);
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

  println!("✓ Received {} items via output channel", output_items.len());
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive only the first item (1) due to throttling
  let expected_output = vec![1i32];
  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ ThrottleNode correctly throttled rapid items, emitting only: {:?}",
      output_items
    );
    println!("  (Other items were dropped due to throttle period)");
  } else {
    println!(
      "⚠ ThrottleNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
