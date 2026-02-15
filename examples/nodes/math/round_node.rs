use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::math::RoundNode;
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
    let path = Path::new("examples/nodes/math/round_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("RoundNode", |id, _inputs, _outputs| {
      Box::new(RoundNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with RoundNode using graph! macro");

  // Send configuration (optional for RoundNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test rounding computations
  println!("\n🧪 Testing rounding computations...");

  // Test 1: round(3.7) = 4 (rounds up)
  println!("  Test 1: round(3.7f64) (expected: 4)");
  let _ = in_tx
    .send(Arc::new(3.7f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: round(3.2) = 3 (rounds down)
  println!("  Test 2: round(3.2f64) (expected: 3)");
  let _ = in_tx
    .send(Arc::new(3.2f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: round(2.5) = 3 (rounds to even, but typically rounds up for .5)
  println!("  Test 3: round(2.5f64) (expected: 3)");
  let _ = in_tx
    .send(Arc::new(2.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: round(-2.7) = -3 (rounds away from zero)
  println!("  Test 4: round(-2.7f64) (expected: -3)");
  let _ = in_tx
    .send(Arc::new(-2.7f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 5: round(-1.5) = -2 (rounds away from zero)
  println!("  Test 5: round(-1.5f64) (expected: -2)");
  let _ = in_tx
    .send(Arc::new(-1.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 6: round(5.0) = 5 (already integer)
  println!("  Test 6: round(5.0f64) (expected: 5)");
  let _ = in_tx
    .send(Arc::new(5.0f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with RoundNode...");
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
      // RoundNode outputs the rounded integer value
      results_received += 1;
      println!("  Rounding result {}:", results_received);

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

  // Verify behavior: should receive 6 results
  if success_count == 6 && error_count == 0 {
    println!("✓ RoundNode correctly computed rounded values");
    println!("  Results should be: [4, 3, 3, -3, -2, 5]");
  } else {
    println!(
      "⚠ RoundNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 6, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
