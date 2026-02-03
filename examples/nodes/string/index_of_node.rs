use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringIndexOfNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (substring_tx, substring_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    index_of: StringIndexOfNode::new("index_of".to_string()),
    graph.configuration => index_of.configuration,
    graph.input => index_of.in,
    graph.substring => index_of.substring,
    index_of.out => graph.output,
    index_of.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("substring", substring_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringIndexOfNode using graph! macro");

  // Send configuration (optional for StringIndexOfNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings and substrings to find indices of
  println!("📥 Sending strings and substrings to find indices");
  let test_input_strings = [
    "Hello World".to_string(),
    "Hello World".to_string(),
    "The quick brown fox".to_string(),
    "The quick brown fox".to_string(),
    "test.txt".to_string(),
    "Rust programming".to_string(),
    "Hello".to_string(),
  ];
  let test_substrings = [
    "World".to_string(),
    "Hello".to_string(),
    "brown".to_string(),
    "slow".to_string(),
    ".txt".to_string(),
    "programming".to_string(),
    "".to_string(),
  ];
  let expected_results = vec![6i64, 0i64, 10i64, -1i64, 4i64, 5i64, 0i64];

  for i in 0..test_input_strings.len() {
    println!(
      "  Finding index of '{}' in '{}' -> expected {}",
      test_substrings[i], test_input_strings[i], expected_results[i]
    );

    input_tx
      .send(Arc::new(test_input_strings[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    substring_tx
      .send(Arc::new(test_substrings[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(substring_tx);

  // Execute the graph
  println!("Executing graph with StringIndexOfNode...");
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

    if let Ok(Some(item)) = output_result
      && let Ok(result) = item.downcast::<i64>()
    {
      output_results.push(*result);
      println!("  Output: {}", *result);
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
    "✓ Received {} results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected index results
  if output_results == expected_results && error_count == 0 {
    println!("✓ StringIndexOfNode correctly found substring indices");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}' in '{}' -> index {}",
        test_substrings[i], test_input_strings[i], result
      );
    }
  } else {
    println!(
      "⚠ StringIndexOfNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
