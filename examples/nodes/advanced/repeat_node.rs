use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::advanced::repeat_node::RepeatNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (count_tx, count_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    repeat: RepeatNode::new("repeat".to_string()),
    graph.configuration => repeat.configuration,
    graph.input => repeat.in,
    graph.count => repeat.count,
    repeat.out => graph.output,
    repeat.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("count", count_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with RepeatNode using graph! macro");

  // Configuration is optional for RepeatNode
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send count first (repeat each item 3 times)
  let _ = count_tx
    .send(Arc::new(3u32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test items to be repeated
  let test_items = vec!["hello", "world", "repeat"];
  for item in test_items {
    let _ = input_tx
      .send(Arc::new(item.to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with RepeatNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(count_tx);

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

    if let Ok(Some(item)) = out_result
      && let Ok(item_arc) = item.downcast::<String>()
    {
      let item_str = (**item_arc).to_string();
      println!("  Output: {}", item_str);
      output_count += 1;
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

  println!(
    "✓ Received {} repeated items via output channel",
    output_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 9 items (3 original × 3 repeats each)
  if output_count == 9 && error_count == 0 {
    println!("✓ RepeatNode correctly repeated each item 3 times");
  } else {
    println!(
      "⚠ RepeatNode behavior may be unexpected (outputs: {}, errors: {}, expected outputs: 9, errors: 0)",
      output_count, error_count
    );
  }

  Ok(())
}
