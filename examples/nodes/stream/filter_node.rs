use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::filter_node::{FilterConfig, FilterNode, filter_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a configuration that filters for even numbers
  let even_filter_config: FilterConfig = filter_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let num = *arc_i32;
      Ok(num % 2 == 0) // Keep even numbers
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Build the graph directly with connected channels
  let mut graph = Graph::new("number_filter".to_string());
  graph.add_node(
    "filter".to_string(),
    Box::new(FilterNode::new("filter".to_string())),
  )?;
  graph.expose_input_port("filter", "configuration", "configuration")?;
  graph.expose_input_port("filter", "in", "input")?;
  graph.expose_output_port("filter", "out", "output")?;
  graph.expose_output_port("filter", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with connected channels");

  // Send configuration and input data to the channels AFTER building
  let _ = config_tx
    .send(Arc::new(even_filter_config) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send mixed numbers (even and odd) to test filtering
  for num in 1..=10 {
    let _ = input_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  println!("✓ Data sent to channels");

  // Execute the graph (channels have data now)
  println!("Executing graph...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);

  // Read results from the output channel
  println!("Reading filtered results from output channel...");
  let mut filtered_count = 0;
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), output_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(arc_i32) = item.downcast::<i32>() {
          let result = *arc_i32;
          println!("  Filtered: {}", result);
          filtered_count += 1;
          if filtered_count >= 5 {
            // We expect 5 even numbers from 1..=10
            break;
          }
        }
      }
      Ok(None) => {
        println!("Output channel closed");
        break;
      }
      Err(_) => {
        // Timeout, continue
      }
    }
  }

  // Read errors from the error channel (should be empty for this example)
  println!("Reading errors from error channel...");
  let mut error_count = 0;
  let mut timeout_count = 0;
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(error_msg) = item.downcast::<String>() {
          let error = &**error_msg;
          println!("  Error: {}", error);
          error_count += 1;
        }
      }
      Ok(None) => {
        println!("Error channel closed");
        break;
      }
      Err(_) => {
        // Timeout, break after a few timeouts since we don't expect errors
        timeout_count += 1;
        if timeout_count >= 5 {
          println!(
            "No more errors (timed out after {} attempts)",
            timeout_count
          );
          break;
        }
      }
    }
  }

  println!(
    "✓ Received {} filtered results via output channel",
    filtered_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
