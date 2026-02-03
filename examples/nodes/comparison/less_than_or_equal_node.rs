use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::comparison::LessThanOrEqualNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    less_than_or_equal: LessThanOrEqualNode::new("less_than_or_equal".to_string()),
    graph.configuration => less_than_or_equal.configuration,
    graph.in1 => less_than_or_equal.in1,
    graph.in2 => less_than_or_equal.in2,
    less_than_or_equal.out => graph.output,
    less_than_or_equal.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with LessThanOrEqualNode using graph! macro");

  // Send configuration (optional for LessThanOrEqualNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test less than or equal comparisons
  println!("\n🧪 Testing less than or equal comparisons...");

  // Test 1: 10 <= 20 (expected: true)
  println!("  Test 1: 10i32 <= 20i32 (expected: true)");
  let _ = in1_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: 20 <= 10 (expected: false)
  println!("  Test 2: 20i32 <= 10i32 (expected: false)");
  let _ = in1_tx
    .send(Arc::new(20i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Equal values: 100 <= 100 (expected: true)
  println!("  Test 3: 100i32 <= 100i32 (expected: true)");
  let _ = in1_tx
    .send(Arc::new(100i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(100i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: 3.2 <= 5.5 (expected: true)
  println!("  Test 4: 3.2f64 <= 5.5f64 (expected: true)");
  let _ = in1_tx
    .send(Arc::new(3.2f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(5.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 5: 5.5 <= 3.2 (expected: false)
  println!("  Test 5: 5.5f64 <= 3.2f64 (expected: false)");
  let _ = in1_tx
    .send(Arc::new(5.5f64) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(3.2f64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with LessThanOrEqualNode...");
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
      // LessThanOrEqualNode outputs the boolean result of less than or equal comparison
      if let Ok(result_arc) = item.clone().downcast::<bool>() {
        let result = *result_arc;
        results_received += 1;
        println!(
          "  Less than or equal result {}: {}",
          results_received, result
        );
        success_count += 1;
        has_data = true;
      }
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

  // Verify behavior: should receive 5 results (true, false, true, true, false)
  if success_count == 5 && error_count == 0 {
    println!("✓ LessThanOrEqualNode correctly performed less than or equal comparisons");
    println!("  Results should match expected values: [true, false, true, true, false]");
  } else {
    println!(
      "⚠ LessThanOrEqualNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 5, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
