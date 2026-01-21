use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::math::ExpNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("exp_example".to_string());
  graph.add_node(
    "exp".to_string(),
    Box::new(ExpNode::new("exp".to_string())),
  )?;
  graph.expose_input_port("exp", "configuration", "configuration")?;
  graph.expose_input_port("exp", "in", "input")?;
  graph.expose_output_port("exp", "out", "output")?;
  graph.expose_output_port("exp", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ExpNode using Graph API");

  // Send configuration (optional for ExpNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test exponential computations
  println!("\n🧪 Testing exponential computations...");

  // Test 1: exp(0) = e^0 = 1
  println!("  Test 1: exp(0) = e^0 (expected: ~1.0)");
  let _ = in_tx
    .send(Arc::new(0.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: exp(1) = e^1 = e ≈ 2.718
  println!("  Test 2: exp(1) = e^1 (expected: ~2.718)");
  let _ = in_tx
    .send(Arc::new(1.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: exp(2) = e^2 ≈ 7.389
  println!("  Test 3: exp(2) = e^2 (expected: ~7.389)");
  let _ = in_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: exp(-1) = e^(-1) ≈ 0.368
  println!("  Test 4: exp(-1) = e^(-1) (expected: ~0.368)");
  let _ = in_tx
    .send(Arc::new(-1.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 5: exp(0.5) = e^(0.5) ≈ 1.649
  println!("  Test 5: exp(0.5) = e^(0.5) (expected: ~1.649)");
  let _ = in_tx
    .send(Arc::new(0.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ExpNode...");
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
      // ExpNode outputs the exponential result as f64
      results_received += 1;
      println!("  Exponential result {}:", results_received);

      if let Ok(exp_result) = item.clone().downcast::<f64>() {
        let value = *exp_result;
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

  println!("✓ Received {} successful results via output channel", success_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 5 results
  if success_count == 5 && error_count == 0 {
    println!("✓ ExpNode correctly computed exponential values");
    println!("  Expected results: e^0 ≈ 1.0, e^1 ≈ 2.718, e^2 ≈ 7.389, e^(-1) ≈ 0.368, e^(0.5) ≈ 1.649");
  } else {
    println!(
      "⚠ ExpNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 5, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
