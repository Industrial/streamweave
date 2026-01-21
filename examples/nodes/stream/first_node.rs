use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::stream::FirstNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("first_example".to_string());
  graph.add_node(
    "first".to_string(),
    Box::new(FirstNode::new("first".to_string())),
  )?;
  graph.expose_input_port("first", "configuration", "configuration")?;
  graph.expose_input_port("first", "in", "input")?;
  graph.expose_output_port("first", "out", "output")?;
  graph.expose_output_port("first", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with FirstNode using Graph API");

  // Send configuration (optional for FirstNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send multiple values: 10, 20, 30, 40, 50
  println!("📥 Sending multiple values: 10, 20, 30, 40, 50");
  let test_values = vec![10i32, 20i32, 30i32, 40i32, 50i32];
  for value in test_values {
    input_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ All test data sent to input channel");

  // Execute the graph
  println!("Executing graph with FirstNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);

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
        println!("  Output: {}", *value);
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

  println!("✓ Received {} items via output channel", output_items.len());
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive only the first item (10)
  let expected_output = vec![10i32];
  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ FirstNode correctly emitted only the first item: {:?}",
      output_items
    );
  } else {
    println!(
      "⚠ FirstNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
