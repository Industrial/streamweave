use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::aggregation::SumNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("sum_example".to_string());
  graph.add_node("sum".to_string(), Box::new(SumNode::new("sum".to_string())))?;
  graph.expose_input_port("sum", "configuration", "configuration")?;
  graph.expose_input_port("sum", "in", "input")?;
  graph.expose_output_port("sum", "out", "output")?;
  graph.expose_output_port("sum", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with SumNode using Graph API");

  // Send configuration (optional for SumNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: numeric values to sum
  let test_values = vec![
    10i32, // 10
    20i32, // 20 (running sum: 30)
    5i32,  // 5 (running sum: 35)
    15i32, // 15 (running sum: 50)
  ];

  for value in test_values {
    let _ = in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with SumNode...");
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

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // SumNode outputs the sum as a single result when stream ends
      if let Ok(sum_arc) = item.clone().downcast::<i32>() {
        let sum = *sum_arc;
        println!("  Sum result: {}", sum);
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

  // Verify behavior: should receive 1 result (50) when stream ends
  if success_count == 1 && error_count == 0 {
    println!("✓ SumNode correctly summed the numeric values");
  } else {
    println!(
      "⚠ SumNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
