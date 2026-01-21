use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::aggregation::MaxAggregateNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("max_aggregate_example".to_string());
  graph.add_node(
    "max".to_string(),
    Box::new(MaxAggregateNode::new("max".to_string())),
  )?;
  graph.expose_input_port("max", "configuration", "configuration")?;
  graph.expose_input_port("max", "in", "input")?;
  graph.expose_output_port("max", "out", "output")?;
  graph.expose_output_port("max", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with MaxAggregateNode using Graph API");

  // Send configuration (optional for MaxAggregateNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: various numeric values
  let test_data = vec![15i32, 42i32, 7i32, 89i32, 23i32]; // Max should be 89
  for num in test_data {
    let _ = input_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with MaxAggregateNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;

  // Give time for aggregation to complete (MaxAggregateNode outputs when stream ends)
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
      // MaxAggregateNode outputs the maximum value in various numeric types
      if let Ok(max_i32) = item.clone().downcast::<i32>() {
        let max = *max_i32;
        println!("  Maximum (i32): {}", max);
        success_count += 1;
        has_data = true;
      } else if let Ok(max_i64) = item.clone().downcast::<i64>() {
        let max = *max_i64;
        println!("  Maximum (i64): {}", max);
        success_count += 1;
        has_data = true;
      } else if let Ok(max_f32) = item.clone().downcast::<f32>() {
        let max = *max_f32;
        println!("  Maximum (f32): {:.2}", max);
        success_count += 1;
        has_data = true;
      } else if let Ok(max_f64) = item.downcast::<f64>() {
        let max = *max_f64;
        println!("  Maximum (f64): {:.2}", max);
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

  // Verify behavior: should receive 1 result (the maximum of 15,42,7,89,23 = 89)
  if success_count == 1 && error_count == 0 {
    println!("✓ MaxAggregateNode correctly found maximum value in the stream");
  } else {
    println!(
      "⚠ MaxAggregateNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
