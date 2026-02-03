use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::advanced::break_node::BreakNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (signal_tx, signal_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    break_node: BreakNode::new("break".to_string()),
    graph.configuration => break_node.configuration,
    graph.in => break_node.in,
    graph.signal => break_node.signal,
    break_node.out => graph.out,
    break_node.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in", input_rx)?;
  graph.connect_input_channel("signal", signal_rx)?;
  graph.connect_output_channel("out", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with BreakNode using graph! macro");

  // Send configuration (empty config for break node)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create a task to send input data and signals concurrently with graph execution
  let input_tx_clone = input_tx.clone();
  let signal_tx_clone = signal_tx.clone();

  tokio::spawn(async move {
    // Send initial input data
    let initial_data = vec!["item1", "item2"];
    for item in initial_data {
      let _ = input_tx_clone
        .send(Arc::new(item.to_string()) as Arc<dyn Any + Send + Sync>)
        .await;
      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Send break signal
    let _ = signal_tx_clone
      .send(Arc::new("BREAK".to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Send more data after break signal (should not be forwarded)
    let post_break_data = vec!["item3", "item4", "item5"];
    for item in post_break_data {
      let _ = input_tx_clone
        .send(Arc::new(item.to_string()) as Arc<dyn Any + Send + Sync>)
        .await;
      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
  });

  println!("✓ Configuration sent and concurrent data/signal sender started");

  // Execute the graph
  println!("Executing graph with BreakNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(signal_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      if let Ok(item_arc) = item.downcast::<String>() {
        let item_str = &**item_arc;
        println!("  Output: {}", item_str);
        output_count += 1;
        has_data = true;
      }
    }

    if let Ok(Some(item)) = error_result {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = &**error_msg;
        println!("  Error: {}", error);
        error_count += 1;
        has_data = true;
      }
    }

    if !has_data {
      break;
    }
  }

  println!("✓ Received {} items via output channel", output_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive only the items sent before break signal
  if output_count == 2 {
    println!("✓ BreakNode correctly stopped forwarding after break signal");
  } else {
    println!(
      "⚠ BreakNode behavior may be unexpected (received {} items, expected 2)",
      output_count
    );
  }

  Ok(())
}
