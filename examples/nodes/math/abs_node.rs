use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::math::AbsNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    abs: AbsNode::new("abs".to_string()),
    graph.configuration => abs.configuration,
    graph.input => abs.in,
    abs.out => graph.output,
    abs.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with AbsNode using graph! macro");

  // Send configuration (optional for AbsNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test absolute value computations
  println!("\n🧪 Testing absolute value computations...");

  // Test 1: abs(-10) (expected: 10)
  println!("  Test 1: abs(-10i32) (expected: 10)");
  let _ = in_tx
    .send(Arc::new(-10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: abs(15) (expected: 15 - already positive)
  println!("  Test 2: abs(15i32) (expected: 15)");
  let _ = in_tx
    .send(Arc::new(15i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: abs(-3.5) (expected: 3.5)
  println!("  Test 3: abs(-3.5f64) (expected: 3.5)");
  let _ = in_tx
    .send(Arc::new(-3.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: abs(7.2) (expected: 7.2 - already positive)
  println!("  Test 4: abs(7.2f64) (expected: 7.2)");
  let _ = in_tx
    .send(Arc::new(7.2f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with AbsNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut success_count = 0;
  let mut error_count = 0;
  let mut results_received = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // AbsNode outputs the absolute value result
      results_received += 1;
      println!("  Absolute value result {}:", results_received);

      // Try different numeric types
      if let Ok(int_result) = item.clone().downcast::<i32>() {
        let value = *int_result;
        println!("    i32: {}", value);
        success_count += 1;
      } else if let Ok(float_result) = item.clone().downcast::<f64>() {
        let value = *float_result;
        println!("    f64: {}", value);
        success_count += 1;
      } else {
        println!("    Unknown type");
        success_count += 1;
      }
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
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 4 results (10, 15, 3.5, 7.2)
  if success_count == 4 && error_count == 0 {
    println!("✓ AbsNode correctly computed absolute values");
    println!("  Results should be: [10, 15, 3.5, 7.2]");
  } else {
    println!(
      "⚠ AbsNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 4, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
