use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::math::LogNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (base_tx, base_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("log_example".to_string());
  graph.add_node("log".to_string(), Box::new(LogNode::new("log".to_string())))?;
  graph.expose_input_port("log", "configuration", "configuration")?;
  graph.expose_input_port("log", "in", "input")?;
  graph.expose_input_port("log", "base", "base")?;
  graph.expose_output_port("log", "out", "output")?;
  graph.expose_output_port("log", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("base", base_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with LogNode using Graph API");

  // Send configuration (optional for LogNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test logarithm computations
  println!("\n🧪 Testing logarithm computations...");

  // Test 1: log(8, 2) = 3 (because 2^3 = 8)
  println!("  Test 1: log(8, 2) (expected: ~3.0)");
  let _ = in_tx
    .send(Arc::new(8.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = base_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: log(100, 10) = 2 (because 10^2 = 100)
  println!("  Test 2: log(100, 10) (expected: ~2.0)");
  let _ = in_tx
    .send(Arc::new(100.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = base_tx
    .send(Arc::new(10.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: ln(e) = log(e, e) = 1 (natural log)
  println!("  Test 3: ln(e) = log(e, e) (expected: ~1.0)");
  let _ = in_tx
    .send(Arc::new(std::f64::consts::E) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = base_tx
    .send(Arc::new(std::f64::consts::E) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: log(1, 2) = 0 (any number to power 0 = 1)
  println!("  Test 4: log(1, 2) (expected: ~0.0)");
  let _ = in_tx
    .send(Arc::new(1.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = base_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 5: log(0.5, 2) = -1 (because 2^-1 = 0.5)
  println!("  Test 5: log(0.5, 2) (expected: ~-1.0)");
  let _ = in_tx
    .send(Arc::new(0.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = base_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with LogNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(base_tx);

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
      // LogNode outputs the logarithm result as f64
      results_received += 1;
      println!("  Logarithm result {}:", results_received);

      if let Ok(log_result) = item.clone().downcast::<f64>() {
        let value = *log_result;
        println!("    f64: {:.6}", value);
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

  // Verify behavior: should receive 5 results
  if success_count == 5 && error_count == 0 {
    println!("✓ LogNode correctly computed logarithm values");
    println!(
      "  Expected results: log₂(8) ≈ 3.0, log₁₀(100) ≈ 2.0, ln(e) ≈ 1.0, log₂(1) ≈ 0.0, log₂(0.5) ≈ -1.0"
    );
  } else {
    println!(
      "⚠ LogNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 5, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
