use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::stream::{FilterMapConfigWrapper, FilterMapNode, filter_map_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (function_tx, function_rx) = mpsc::channel(1);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    filter_map: FilterMapNode::new("filter_map".to_string()),
    graph.configuration => filter_map.configuration,
    graph.input => filter_map.in,
    graph.function => filter_map.function,
    filter_map.out => graph.output,
    filter_map.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("function", function_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with FilterMapNode using graph! macro");

  // Send configuration (optional for FilterMapNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send filter-map function: filter even numbers and double them
  let filter_double_config = filter_map_config(|value| async move {
    if let Ok(i32_val) = value.downcast::<i32>() {
      if *i32_val % 2 == 0 {
        // Even number: double it and emit
        Ok(Some(Arc::new(*i32_val * 2) as Arc<dyn Any + Send + Sync>))
      } else {
        // Odd number: filter out
        Ok(None)
      }
    } else {
      Err("Expected i32 values".to_string())
    }
  });
  function_tx
    .send(Arc::new(FilterMapConfigWrapper::new(filter_double_config)) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  println!("📥 Sending filter-map function: filter evens and double them");

  // Send test data: 1, 2, 3, 4, 5, 6, 7, 8
  println!("📥 Sending values: 1, 2, 3, 4, 5, 6, 7, 8");
  let test_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32, 8i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channel");

  // Execute the graph
  println!("Executing graph with FilterMapNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(function_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(1000), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(1000), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result
      && let Ok(value) = item.downcast::<i32>()
    {
      output_items.push(*value);
      println!("  Output: {}", *value);
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

  println!("✓ Received {} items via output channel", output_items.len());
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive doubled even numbers: 4, 8, 12, 16
  let expected_output = vec![4i32, 8i32, 12i32, 16i32];
  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ FilterMapNode correctly filtered odd numbers and doubled even numbers: {:?}",
      output_items
    );
  } else {
    println!(
      "⚠ FilterMapNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
