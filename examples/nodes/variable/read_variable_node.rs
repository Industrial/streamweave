#![allow(clippy::approx_constant)]
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::read_variable_node::{ReadVariableConfig, ReadVariableNode};
use tokio::sync::{Mutex, mpsc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (name_tx, name_rx) = mpsc::channel(5);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a variable store and populate it with some variables
  let variable_store: ReadVariableConfig = Arc::new(Mutex::new(HashMap::new()));

  // Populate the store with some test variables
  {
    let mut store = variable_store.lock().await;
    store.insert(
      "user_count".to_string(),
      Arc::new(42i32) as Arc<dyn Any + Send + Sync>,
    );
    store.insert(
      "server_name".to_string(),
      Arc::new("production-server".to_string()) as Arc<dyn Any + Send + Sync>,
    );
    store.insert(
      "pi_value".to_string(),
      Arc::new(3.14159f64) as Arc<dyn Any + Send + Sync>,
    );
    store.insert(
      "is_active".to_string(),
      Arc::new(true) as Arc<dyn Any + Send + Sync>,
    );
  }

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    read_var: ReadVariableNode::new("read_var".to_string()),
    graph.configuration => read_var.configuration,
    graph.name => read_var.name,
    read_var.out => graph.out,
    read_var.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("name", name_rx)?;
  graph.connect_output_channel("out", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with graph! macro");

  // Send configuration and input data to the channels AFTER building
  let _ = config_tx
    .send(Arc::new(variable_store) as Arc<dyn Any + Send + Sync>)
    .await;

  let variables_to_read = vec![
    "user_count",      // exists - should return 42
    "server_name",     // exists - should return "production-server"
    "nonexistent",     // doesn't exist - should error
    "pi_value",        // exists - should return 3.14159
    "another_missing", // doesn't exist - should error
  ];

  for var_name in variables_to_read {
    let _ = name_tx
      .send(Arc::new(var_name.to_string()) as Arc<dyn Any + Send + Sync>)
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
  drop(name_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut value_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // Try to downcast to different types
      if let Ok(int_arc) = item.clone().downcast::<i32>() {
        let value = *int_arc;
        println!("  Variable value (i32): {}", value);
        value_count += 1;
        has_data = true;
      } else if let Ok(string_arc) = item.clone().downcast::<String>() {
        let value = &**string_arc;
        println!("  Variable value (String): {}", value);
        value_count += 1;
        has_data = true;
      } else if let Ok(float_arc) = item.clone().downcast::<f64>() {
        let value = *float_arc;
        println!("  Variable value (f64): {}", value);
        value_count += 1;
        has_data = true;
      } else if let Ok(bool_arc) = item.downcast::<bool>() {
        let value = *bool_arc;
        println!("  Variable value (bool): {}", value);
        value_count += 1;
        has_data = true;
      }
    }

    if let Ok(Some(item)) = error_result
      && let Ok(error_msg) = item.downcast::<String>()
    {
      let error = &**error_msg;
      println!("  Error: {}", error);
      error_count += 1;
      has_data = true;
    }

    if !has_data {
      break;
    }
  }

  println!(
    "✓ Received {} variable values via output channel",
    value_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
