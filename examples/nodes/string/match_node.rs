use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::match_node::{MatchConfig, MatchNode, match_exact_string};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(10);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (out0_tx, mut out0_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (out1_tx, mut out1_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (out2_tx, mut out2_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (default_tx, mut default_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    match_node: MatchNode::new("match".to_string(), 3),
    graph.configuration => match_node.configuration,
    graph.input => match_node.in,
    match_node.out_0 => graph.output0,
    match_node.out_1 => graph.output1,
    match_node.out_2 => graph.output2,
    match_node.default => graph.default,
    match_node.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output0", out0_tx)?;
  graph.connect_output_channel("output1", out1_tx)?;
  graph.connect_output_channel("output2", out2_tx)?;
  graph.connect_output_channel("default", default_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with MatchNode using graph! macro");

  // Create a configuration that routes strings to different output ports
  let string_config: MatchConfig = match_exact_string(vec![
    ("error", 0),   // Route to out_0
    ("warning", 1), // Route to out_1
    ("info", 2),    // Route to out_2
  ]);

  // Send the configuration
  let _ = config_tx
    .send(Arc::new(string_config) as Arc<dyn Any + Send + Sync>)
    .await;

  let test_strings = vec![
    "error",   // Should go to out_0
    "warning", // Should go to out_1
    "info",    // Should go to out_2
    "debug",   // Should go to default (no match)
    "trace",   // Should go to default (no match)
  ];

  for s in test_strings {
    let _ = input_tx
      .send(Arc::new(s.to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Send a non-string to demonstrate error handling
  let _ = input_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration and input data sent to streams");

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with MatchNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut total_processed = 0;

  // Read from output0 channel
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), out0_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(string_arc) = item.downcast::<String>() {
          let value = &**string_arc;
          println!("  out_0: {}", value);
          total_processed += 1;
        }
      }
      Ok(None) => break,
      Err(_) => break,
    }
  }

  // Read from output1 channel
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), out1_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(string_arc) = item.downcast::<String>() {
          let value = &**string_arc;
          println!("  out_1: {}", value);
          total_processed += 1;
        }
      }
      Ok(None) => break,
      Err(_) => break,
    }
  }

  // Read from output2 channel
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), out2_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(string_arc) = item.downcast::<String>() {
          let value = &**string_arc;
          println!("  out_2: {}", value);
          total_processed += 1;
        }
      }
      Ok(None) => break,
      Err(_) => break,
    }
  }

  // Read from default channel
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), default_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(string_arc) = item.downcast::<String>() {
          let value = &**string_arc;
          println!("  default: {}", value);
          total_processed += 1;
        }
      }
      Ok(None) => break,
      Err(_) => break,
    }
  }

  // Read errors from the error channel
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(error_msg) = item.downcast::<String>() {
          let error = &**error_msg;
          println!("  error: {}", error);
          total_processed += 1;
        }
      }
      Ok(None) => break,
      Err(_) => break,
    }
  }

  println!(
    "✓ Received {} items routed to branches via output ports",
    total_processed
  );
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
