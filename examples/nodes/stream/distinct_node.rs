use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::stream::DistinctNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    distinct: DistinctNode::new("distinct".to_string()),
    graph.configuration => distinct.configuration,
    graph.input => distinct.in,
    distinct.out => graph.output,
    distinct.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with DistinctNode using graph! macro");

  // Send configuration (optional for DistinctNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: items with duplicates that should be filtered
  println!("📥 Sending values with duplicates: A, B, A, C, B, D");
  let test_values = vec!["A", "B", "A", "C", "B", "D"];
  for value in test_values {
    input_tx
      .send(Arc::new(value.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channel");

  // Execute the graph
  println!("Executing graph with DistinctNode...");
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
      if let Ok(value) = item.downcast::<String>() {
        output_items.push(value.clone());
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

  // Verify behavior: should receive only unique items A, B, C, D
  let expected_values = vec!["A", "B", "C", "D"];

  if output_items.len() == expected_values.len() && error_count == 0 {
    let mut all_correct = true;
    for (i, item) in output_items.iter().enumerate() {
      if item.as_str() != expected_values[i] {
        all_correct = false;
        break;
      }
    }

    if all_correct {
      println!(
        "✓ DistinctNode correctly filtered duplicates, emitting only unique values: {:?}",
        output_items
          .iter()
          .map(|s| s.as_str())
          .collect::<Vec<&str>>()
      );
    } else {
      println!("⚠ Output values do not match expected unique values");
    }
  } else {
    println!(
      "⚠ Unexpected behavior (outputs: {}, expected: {}, errors: {})",
      output_items.len(),
      expected_values.len(),
      error_count
    );
  }

  Ok(())
}
