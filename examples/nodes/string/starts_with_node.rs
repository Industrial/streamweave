use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringStartsWithNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (prefix_tx, prefix_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    starts_with: StringStartsWithNode::new("starts_with".to_string()),
    graph.configuration => starts_with.configuration,
    graph.input => starts_with.in,
    graph.prefix => starts_with.prefix,
    starts_with.out => graph.output,
    starts_with.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("prefix", prefix_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringStartsWithNode using graph! macro");

  // Send configuration (optional for StringStartsWithNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings and prefixes to check
  println!("📥 Sending strings and prefixes to check");
  let test_input_strings = vec![
    "Hello World".to_string(),
    "Rust Programming".to_string(),
    "Test Case".to_string(),
    "example".to_string(),
    "".to_string(),
    "single".to_string(),
    "prefix test".to_string(),
    "no match".to_string(),
  ];
  let test_prefixes = vec![
    "Hello".to_string(),
    "Python".to_string(),
    "Test".to_string(),
    "ex".to_string(),
    "".to_string(),
    "single".to_string(),
    "prefix".to_string(),
    "xyz".to_string(),
  ];

  for i in 0..test_input_strings.len() {
    println!(
      "  Checking if '{}' starts with '{}' -> expected {}",
      test_input_strings[i],
      test_prefixes[i],
      test_input_strings[i].starts_with(&test_prefixes[i])
    );

    input_tx
      .send(Arc::new(test_input_strings[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    prefix_tx
      .send(Arc::new(test_prefixes[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(prefix_tx);

  // Execute the graph
  println!("Executing graph with StringStartsWithNode...");
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
      if let Ok(result_bool) = item.downcast::<bool>() {
        output_results.push(*result_bool);
        println!("  Output: {}", *result_bool);
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

  // Verify behavior: should receive expected boolean results
  let expected_results = vec![
    true,  // "Hello World" starts with "Hello"
    false, // "Rust Programming" does not start with "Python"
    true,  // "Test Case" starts with "Test"
    true,  // "example" starts with "ex"
    true,  // "" starts with "" (empty string case)
    true,  // "single" starts with "single"
    true,  // "prefix test" starts with "prefix"
    false, // "no match" does not start with "xyz"
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringStartsWithNode correctly checked string prefixes");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}' starts with '{}' -> {}",
        test_input_strings[i], test_prefixes[i], result
      );
    }
  } else {
    println!(
      "⚠ StringStartsWithNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
