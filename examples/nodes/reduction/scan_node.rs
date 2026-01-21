use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::reduction::{ScanNode, scan_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (initial_tx, initial_rx) = mpsc::channel(1);
  let (function_tx, function_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("scan_example".to_string());
  graph.add_node(
    "scan".to_string(),
    Box::new(ScanNode::new("scan".to_string())),
  )?;
  graph.expose_input_port("scan", "configuration", "configuration")?;
  graph.expose_input_port("scan", "in", "input")?;
  graph.expose_input_port("scan", "initial", "initial")?;
  graph.expose_input_port("scan", "function", "function")?;
  graph.expose_output_port("scan", "out", "output")?;
  graph.expose_output_port("scan", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("initial", initial_rx)?;
  graph.connect_input_channel("function", function_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ScanNode using Graph API");

  // Send configuration (optional for ScanNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send initial value
  let initial_value = 0i32;
  initial_tx
    .send(Arc::new(initial_value) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending initial value: {}", initial_value);

  // Send scan function (sum)
  let sum_config = scan_config(
    |acc: Arc<dyn Any + Send + Sync>, value: Arc<dyn Any + Send + Sync>| async move {
      if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
        Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn Any + Send + Sync>)
      } else {
        Err("Expected i32 values for sum operation".to_string())
      }
    },
  );
  function_tx
    .send(
      Arc::new(streamweave::nodes::reduction::ScanConfigWrapper::new(
        sum_config,
      )) as Arc<dyn Any + Send + Sync>,
    )
    .await
    .unwrap();
  println!("📥 Sending scan function: sum");

  // Send test data: 6 items to demonstrate scan behavior
  println!("📥 Sending values to scan: 1, 2, 3, 4, 5, 6");
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ScanNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(initial_tx);
  drop(function_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result {
      if let Ok(value) = item.downcast::<i32>() {
        output_items.push(*value);
        has_data = true;
        println!("  Output: {}", *value);
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

  println!("✓ Received {} items via output channel", output_items.len());
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive intermediate scan results
  // Initial: 0, then 0+1=1, 1+2=3, 3+3=6, 6+4=10, 10+5=15, 15+6=21
  let expected_output = vec![0i32, 1i32, 3i32, 6i32, 10i32, 15i32, 21i32];
  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ ScanNode correctly emitted intermediate results: {:?}",
      output_items
    );
  } else {
    println!(
      "⚠ ScanNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
