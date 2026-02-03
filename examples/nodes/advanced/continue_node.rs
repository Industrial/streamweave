use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::advanced::continue_node::ContinueNode;
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
    continue_node: ContinueNode::new("continue".to_string()),
    graph.configuration => continue_node.configuration,
    graph.input => continue_node.in,
    graph.signal => continue_node.signal,
    continue_node.out => graph.output,
    continue_node.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("signal", signal_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ContinueNode using graph! macro");

  // Send configuration (empty config for continue node)
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

    // Send continue signal - this should skip the next item
    let _ = signal_tx_clone
      .send(Arc::new("CONTINUE".to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Send more data - the first one after signal should be skipped
    let post_signal_data = vec!["item3", "item4", "item5"];
    for item in post_signal_data {
      let _ = input_tx_clone
        .send(Arc::new(item.to_string()) as Arc<dyn Any + Send + Sync>)
        .await;
      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
  });

  println!("✓ Configuration sent and concurrent data/signal sender started");

  // Execute the graph
  println!("Executing graph with ContinueNode...");
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
  let mut received_items = Vec::new();
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result
      && let Ok(item_arc) = item.downcast::<String>()
    {
      let item_str = (**item_arc).to_string();
      println!("  Output: {}", item_str);
      received_items.push(item_str);
      output_count += 1;
      has_data = true;
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

  println!("✓ Received {} items via output channel", output_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive item1, item2, item4, item5 (item3 should be skipped)
  let expected_items = vec!["item1", "item2", "item4", "item5"];
  if received_items == expected_items {
    println!("✓ ContinueNode correctly skipped item after continue signal");
  } else {
    println!(
      "⚠ ContinueNode behavior may be unexpected (received: {:?}, expected: {:?})",
      received_items, expected_items
    );
  }

  Ok(())
}
