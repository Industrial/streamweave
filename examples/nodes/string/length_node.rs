use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringLengthNode;
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
    length: StringLengthNode::new("length".to_string()),
    graph.configuration => length.configuration,
    graph.input => length.in,
    length.out => graph.output,
    length.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringLengthNode using graph! macro");

  // Send configuration (optional for StringLengthNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test strings to measure lengths
  println!("📥 Sending strings to measure lengths");
  let test_strings = vec![
    "Hello".to_string(),
    "Hello World".to_string(),
    "".to_string(),
    "🌍🌎🌏".to_string(),
    "a".to_string(),
    "StreamWeave".to_string(),
  ];

  for test_str in &test_strings {
    println!(
      "  Measuring length of: '{}' (expected: {})",
      test_str,
      test_str.len()
    );
    input_tx
      .send(Arc::new(test_str.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with StringLengthNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_results = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result {
      if let Ok(result) = item.downcast::<usize>() {
        output_results.push(*result);
        println!("  Output: {}", *result);
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

  println!(
    "✓ Received {} results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected length results
  let expected_results = vec![5usize, 11usize, 0usize, 9usize, 1usize, 11usize];
  if output_results == expected_results && error_count == 0 {
    println!("✓ StringLengthNode correctly measured string lengths");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!("    '{}' -> length {}", test_strings[i], result);
    }
    println!("  Note: Unicode characters like 🌍 count as multiple bytes (3 each)");
  } else {
    println!(
      "⚠ StringLengthNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
