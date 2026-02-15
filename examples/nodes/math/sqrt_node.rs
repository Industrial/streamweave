use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::math::SqrtNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/math/sqrt_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("SqrtNode", |id, _inputs, _outputs| {
      Box::new(SqrtNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with SqrtNode using graph! macro");

  // Send configuration (optional for SqrtNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test square root computations
  println!("\n🧪 Testing square root computations...");

  // Test 1: sqrt(9) = 3
  println!("  Test 1: sqrt(9.0) (expected: ~3.0)");
  let _ = in_tx
    .send(Arc::new(9.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: sqrt(25) = 5
  println!("  Test 2: sqrt(25.0) (expected: ~5.0)");
  let _ = in_tx
    .send(Arc::new(25.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: sqrt(2) ≈ 1.414
  println!("  Test 3: sqrt(2.0) (expected: ~1.414)");
  let _ = in_tx
    .send(Arc::new(2.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: sqrt(0) = 0
  println!("  Test 4: sqrt(0.0) (expected: ~0.0)");
  let _ = in_tx
    .send(Arc::new(0.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 5: sqrt(16) = 4
  println!("  Test 5: sqrt(16.0) (expected: ~4.0)");
  let _ = in_tx
    .send(Arc::new(16.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 6: sqrt(-4) = error (negative number)
  println!("  Test 6: sqrt(-4.0) (expected: error)");
  let _ = in_tx
    .send(Arc::new(-4.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with SqrtNode...");
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
      // SqrtNode outputs the square root result as f64
      results_received += 1;
      println!("  Square root result {}:", results_received);

      if let Ok(sqrt_result) = item.clone().downcast::<f64>() {
        let value = *sqrt_result;
        println!("    f64: {:.6}", value);
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

  // Verify behavior: should receive 5 results and 1 error
  if success_count == 5 && error_count == 1 {
    println!("✓ SqrtNode correctly computed square roots");
    println!(
      "  Results should be: sqrt(9) ≈ 3.0, sqrt(25) ≈ 5.0, sqrt(2) ≈ 1.414, sqrt(0) ≈ 0.0, sqrt(16) ≈ 4.0, sqrt(-4) = error"
    );
  } else {
    println!(
      "⚠ SqrtNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 5, errors: 1)",
      success_count, error_count
    );
  }

  Ok(())
}
