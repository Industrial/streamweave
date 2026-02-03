use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::arithmetic::SubtractNode;
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
    subtract: SubtractNode::new("subtract".to_string()),
    graph.configuration => subtract.configuration,
    graph.in1 => subtract.in1,
    graph.in2 => subtract.in2,
    subtract.out => graph.output,
    subtract.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with SubtractNode using graph! macro");

  // Send configuration (optional for SubtractNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of numbers to subtract
  let test_pairs = vec![(20i32, 5i32), (50i32, 15i32), (100i32, 25i32)]; // Results: 15, 35, 75
  for (minuend, subtrahend) in test_pairs {
    let _ = in1_tx
      .send(Arc::new(minuend) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = in2_tx
      .send(Arc::new(subtrahend) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with SubtractNode...");
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

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // SubtractNode outputs the difference in various numeric types
      if let Ok(difference_i32) = item.clone().downcast::<i32>() {
        let difference = *difference_i32;
        println!("  Difference (i32): {}", difference);
        success_count += 1;
        has_data = true;
      } else if let Ok(difference_i64) = item.clone().downcast::<i64>() {
        let difference = *difference_i64;
        println!("  Difference (i64): {}", difference);
        success_count += 1;
        has_data = true;
      } else if let Ok(difference_f32) = item.clone().downcast::<f32>() {
        let difference = *difference_f32;
        println!("  Difference (f32): {:.2}", difference);
        success_count += 1;
        has_data = true;
      } else if let Ok(difference_f64) = item.downcast::<f64>() {
        let difference = *difference_f64;
        println!("  Difference (f64): {:.2}", difference);
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

  // Verify behavior: should receive 3 results (20-5=15, 50-15=35, 100-25=75)
  if success_count == 3 && error_count == 0 {
    println!("✓ SubtractNode correctly performed subtraction operations");
  } else {
    println!(
      "⚠ SubtractNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
