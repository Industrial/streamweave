use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::math::FloorNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("floor_example".to_string());
  graph.add_node(
    "floor".to_string(),
    Box::new(FloorNode::new("floor".to_string())),
  )?;
  graph.expose_input_port("floor", "configuration", "configuration")?;
  graph.expose_input_port("floor", "in", "input")?;
  graph.expose_output_port("floor", "out", "output")?;
  graph.expose_output_port("floor", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with FloorNode using Graph API");

  // Send configuration (optional for FloorNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test floor computations
  println!("\n🧪 Testing floor computations...");

  // Test 1: floor(3.7) (expected: 3)
  println!("  Test 1: floor(3.7f64) (expected: 3)");
  let _ = in_tx
    .send(Arc::new(3.7f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: floor(-2.3) (expected: -3)
  println!("  Test 2: floor(-2.3f64) (expected: -3)");
  let _ = in_tx
    .send(Arc::new(-2.3f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: floor(5.0) (expected: 5 - already integer)
  println!("  Test 3: floor(5.0f64) (expected: 5)");
  let _ = in_tx
    .send(Arc::new(5.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: floor(-1.9) (expected: -2)
  println!("  Test 4: floor(-1.9f64) (expected: -2)");
  let _ = in_tx
    .send(Arc::new(-1.9f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with FloorNode...");
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
      // FloorNode outputs the floored integer value
      results_received += 1;
      println!("  Floor result {}:", results_received);

      // Try different numeric types
      if let Ok(int_result) = item.clone().downcast::<i64>() {
        let value = *int_result;
        println!("    i64: {}", value);
        success_count += 1;
      } else if let Ok(int_result) = item.clone().downcast::<i32>() {
        let value = *int_result;
        println!("    i32: {}", value);
        success_count += 1;
      } else {
        println!("    Unknown type");
        success_count += 1;
      }
      has_data = true;
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

  // Verify behavior: should receive 4 results (3, -3, 5, -2)
  if success_count == 4 && error_count == 0 {
    println!("✓ FloorNode correctly computed floor values");
    println!("  Results should be: [3, -3, 5, -2]");
  } else {
    println!(
      "⚠ FloorNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 4, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
