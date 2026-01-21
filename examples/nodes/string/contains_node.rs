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

  // Test cases: string and substrings to check
  let test_cases = vec![
    ("Hello World", "World", true),
    ("Hello World", "world", false), // case sensitive
    ("The quick brown fox", "quick", true),
    ("The quick brown fox", "slow", false),
  ];

  println!("📥 Testing string contains functionality");

  for (input_str, substring, expected) in test_cases {
    println!(
      "  Testing: '{}' contains '{}' -> expected {}",
      input_str, substring, expected
    );

    // Send input string and substring
    input_tx
      .send(Arc::new(input_str.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    substring_tx
      .send(Arc::new(substring.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    // Small delay to ensure processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Test data sent to input channels");

  // Execute the graph
  println!("Executing graph with StringContainsNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(substring_tx);

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
