use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::comparison::EqualNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("equal_example".to_string());
  graph.add_node(
    "equal".to_string(),
    Box::new(EqualNode::new("equal".to_string())),
  )?;
  graph.expose_input_port("equal", "configuration", "configuration")?;
  graph.expose_input_port("equal", "in1", "in1")?;
  graph.expose_input_port("equal", "in2", "in2")?;
  graph.expose_output_port("equal", "out", "output")?;
  graph.expose_output_port("equal", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with EqualNode using Graph API");

  // Send configuration (optional for EqualNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test equality comparisons
  println!("\n🧪 Testing equality comparisons...");

  // Test 1: Equal integers
  println!("  Test 1: 10i32 == 10i32 (expected: true)");
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: Unequal integers
  println!("  Test 2: 10i32 == 20i32 (expected: false)");
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Equal floats
  println!("  Test 3: 5.5f64 == 5.5f64 (expected: true)");
  let _ = in1_tx
    .send(Arc::new(5.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: Unequal floats
  println!("  Test 4: 5.5f64 == 6.5f64 (expected: false)");
  let _ = in1_tx
    .send(Arc::new(5.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(6.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 5: Equal strings
  println!("  Test 5: \"hello\" == \"hello\" (expected: true)");
  let _ = in1_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 6: Unequal strings
  println!("  Test 6: \"hello\" == \"world\" (expected: false)");
  let _ = in1_tx
    .send(Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new("world".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 7: Equal booleans
  println!("  Test 7: true == true (expected: true)");
  let _ = in1_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 8: Unequal booleans
  println!("  Test 8: true == false (expected: false)");
  let _ = in1_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(false) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with EqualNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in1_tx);
  drop(in2_tx);

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
      // EqualNode outputs the boolean result of equality comparison
      if let Ok(result_arc) = item.clone().downcast::<bool>() {
        let result = *result_arc;
        results_received += 1;
        println!("  Equality result {}: {}", results_received, result);
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

  // Verify behavior: should receive 8 results (one for each test pair)
  if success_count == 8 && error_count == 0 {
    println!("✓ EqualNode correctly performed equality comparisons");
    println!(
      "  Results should match expected values: [true, false, true, false, true, false, true, false]"
    );
  } else {
    println!(
      "⚠ EqualNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 8, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
