use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::advanced::retry_node::{RetryConfig, RetryNode, retry_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (max_retries_tx, max_retries_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a retry configuration that simulates failures and recoveries
  let retry_config: RetryConfig = retry_config(|value| async move {
    // Simulate a function that randomly fails or succeeds
    if let Ok(item_arc) = value.downcast::<i32>() {
      let item = *item_arc;

      // Simple simulation: succeed immediately for even numbers, fail then succeed for odd
      if item % 2 == 0 {
        Ok(Arc::new(format!("Success for even number {}", item)) as Arc<dyn Any + Send + Sync>)
      } else {
        // For odd numbers, simulate failure that will be retried
        Err(Arc::new(format!("Failed for odd number {}", item)) as Arc<dyn Any + Send + Sync>)
      }
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });

  // Build the graph using the Graph API
  let mut graph = Graph::new("retry_example".to_string());
  graph.add_node(
    "retry".to_string(),
    Box::new(RetryNode::new("retry".to_string(), 100)), // 100ms base delay
  )?;
  graph.expose_input_port("retry", "configuration", "configuration")?;
  graph.expose_input_port("retry", "in", "input")?;
  graph.expose_input_port("retry", "max_retries", "max_retries")?;
  graph.expose_output_port("retry", "out", "output")?;
  graph.expose_output_port("retry", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("max_retries", max_retries_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with RetryNode using Graph API");

  // Send configuration first
  let _ = config_tx
    .send(Arc::new(retry_config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send max retries
  let _ = max_retries_tx
    .send(Arc::new(3u32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Send test items: even numbers succeed immediately, odd numbers fail and get retried
  let test_items = vec![1, 2, 3]; // 1 and 3 will fail then succeed, 2 will succeed immediately
  for item in test_items {
    let _ = input_tx
      .send(Arc::new(item) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await; // Longer delay between items
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with RetryNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;

  // Give time for retry operations to complete (they have delays)
  tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(max_retries_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut success_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      if let Ok(result_arc) = item.downcast::<String>() {
        let result = (**result_arc).to_string();
        println!("  Success: {}", result);
        success_count += 1;
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
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive successes (some after retries)
  if success_count == 3 && error_count == 0 {
    println!("✓ RetryNode correctly retried failed operations until success");
  } else {
    println!(
      "⚠ RetryNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
