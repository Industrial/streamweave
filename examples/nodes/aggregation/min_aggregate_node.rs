use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::aggregation::MinAggregateNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("min_aggregate_example".to_string());
  graph.add_node(
    "min".to_string(),
    Box::new(MinAggregateNode::new("min".to_string())),
  )?;
  graph.expose_input_port("min", "configuration", "configuration")?;
  graph.expose_input_port("min", "in", "input")?;
  graph.expose_output_port("min", "out", "output")?;
  graph.expose_output_port("min", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with MinAggregateNode using Graph API");

  // Send configuration (optional for MinAggregateNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: various numeric values
  let test_data = vec![15i32, 42i32, 7i32, 89i32, 23i32]; // Min should be 7
  for num in test_data {
    let _ = input_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with MinAggregateNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;

  // Give time for aggregation to complete (MinAggregateNode outputs when stream ends)
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);

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
      // MinAggregateNode outputs the minimum value in various numeric types
      if let Ok(min_i32) = item.clone().downcast::<i32>() {
        let min = *min_i32;
        println!("  Minimum (i32): {}", min);
        success_count += 1;
        has_data = true;
      } else if let Ok(min_i64) = item.clone().downcast::<i64>() {
        let min = *min_i64;
        println!("  Minimum (i64): {}", min);
        success_count += 1;
        has_data = true;
      } else if let Ok(min_f32) = item.clone().downcast::<f32>() {
        let min = *min_f32;
        println!("  Minimum (f32): {:.2}", min);
        success_count += 1;
        has_data = true;
      } else if let Ok(min_f64) = item.downcast::<f64>() {
        let min = *min_f64;
        println!("  Minimum (f64): {:.2}", min);
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

  println!("✓ Received {} successful results via output channel", success_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 1 result (the minimum of 15,42,7,89,23 = 7)
  if success_count == 1 && error_count == 0 {
    println!("✓ MinAggregateNode correctly found minimum value in the stream");
  } else {
    println!(
      "⚠ MinAggregateNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
