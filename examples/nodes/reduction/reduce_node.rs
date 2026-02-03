//! Example demonstrating ReduceNode usage with Graph API
//!
//! This example shows how to use ReduceNode to sum a stream of numbers.
//! The ReduceNode applies a reduction function to accumulate values from a stream.
//!
//! Ports used:
//! - configuration: Not used in this example (for consistency)
//! - initial: Initial accumulator value (0)
//! - function: Reduction function configuration (sum function)
//! - in: Input stream of values to reduce (1, 2, 3)
//! - out: Final accumulated result (6)
//! - error: Any errors that occur

use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::reduction::{ReduceConfigWrapper, ReduceNode, reduce_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (initial_tx, initial_rx) = mpsc::channel(10);
  let (function_tx, function_rx) = mpsc::channel(10);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    reduce: ReduceNode::new("reduce".to_string()),
    graph.configuration => reduce.configuration,
    graph.initial => reduce.initial,
    graph.function => reduce.function,
    graph.in => reduce.in,
    reduce.out => graph.output,
    reduce.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("initial", initial_rx)?;
  graph.connect_input_channel("function", function_rx)?;
  graph.connect_input_channel("in", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ReduceNode using graph! macro");

  // Create a sum reduction function
  let sum_function = reduce_config(
    |acc: Arc<dyn std::any::Any + Send + Sync>, value: Arc<dyn std::any::Any + Send + Sync>| async move {
      if let (Ok(acc_i32), Ok(val_i32)) = (acc.downcast::<i32>(), value.downcast::<i32>()) {
        Ok(Arc::new(*acc_i32 + *val_i32) as Arc<dyn std::any::Any + Send + Sync>)
      } else {
        Err("Expected i32 values for sum".to_string())
      }
    },
  );

  // Send configuration (optional for ReduceNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send the initial accumulator value (0)
  println!("📥 Sending initial value: 0");
  initial_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send the reduction function
  println!("📥 Sending sum reduction function");
  function_tx
    .send(Arc::new(ReduceConfigWrapper::new(sum_function)) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Send values to be summed: 1, 2, 3
  println!("📥 Sending values to sum: 1, 2, 3");
  let test_values = vec![1i32, 2i32, 3i32]; // Expected result: 6 (0 + 1 + 2 + 3)
  for value in test_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ReduceNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(initial_tx);
  drop(function_tx);
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
      // ReduceNode outputs the final accumulated result
      if let Ok(sum) = item.downcast::<i32>() {
        let result = *sum;
        println!("  Final result: {}", result);
        success_count += 1;
        has_data = true;
        // Verify the result
        if result == 6 {
          println!("  ✓ Correct result: 0 + 1 + 2 + 3 = 6");
        } else {
          println!("  ⚠ Unexpected result (expected 6, got {})", result);
        }
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

  // Verify behavior: should receive 1 result (6) and 0 errors
  if success_count == 1 && error_count == 0 {
    println!("✓ ReduceNode correctly performed reduction operations");
  } else {
    println!(
      "⚠ ReduceNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
