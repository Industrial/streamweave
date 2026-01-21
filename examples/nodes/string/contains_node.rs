use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringContainsNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (substring_tx, substring_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("contains_example".to_string());
  graph.add_node(
    "contains".to_string(),
    Box::new(StringContainsNode::new("contains".to_string())),
  )?;
  graph.expose_input_port("contains", "configuration", "configuration")?;
  graph.expose_input_port("contains", "in", "input")?;
  graph.expose_input_port("contains", "substring", "substring")?;
  graph.expose_output_port("contains", "out", "output")?;
  graph.expose_output_port("contains", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("substring", substring_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringContainsNode using Graph API");

  // Send configuration (optional for StringContainsNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings and substrings to check
  println!("📥 Sending test strings and substrings");
  let test_strings = vec![
    "Hello World".to_string(),
    "Hello World".to_string(),
    "The quick brown fox".to_string(),
    "The quick brown fox".to_string(),
  ];
  let test_substrings = vec![
    "World".to_string(),
    "world".to_string(),
    "quick".to_string(),
    "slow".to_string(),
  ];

  for i in 0..test_strings.len() {
    input_tx
      .send(Arc::new(test_strings[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    substring_tx
      .send(Arc::new(test_substrings[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    println!(
      "  Sent: '{}' contains '{}' -> expected {}",
      test_strings[i],
      test_substrings[i],
      test_strings[i].contains(&test_substrings[i])
    );
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(substring_tx);

  // Execute the graph
  println!("Executing graph with StringContainsNode...");
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
      tokio::time::timeout(tokio::time::Duration::from_millis(100), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result {
      if let Ok(result) = item.downcast::<bool>() {
        output_results.push(*result);
        has_data = true;
        println!("  Output: {}", *result);
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

  // Verify behavior: should receive expected boolean results
  let expected_results = vec![true, false, true, false];
  if output_results == expected_results && error_count == 0 {
    println!("✓ StringContainsNode correctly checked substring containment");
  } else {
    println!(
      "⚠ StringContainsNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
