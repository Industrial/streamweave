use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::ToBooleanNode;
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
    to_boolean: ToBooleanNode::new("to_boolean".to_string()),
    graph.configuration => to_boolean.configuration,
    graph.input => to_boolean.in,
    to_boolean.out => graph.output,
    to_boolean.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ToBooleanNode using graph! macro");

  // Send configuration (optional for ToBooleanNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to convert to boolean
  println!("📥 Sending various values for boolean conversion");
  let test_cases = vec![
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    (
      "Boolean false",
      Arc::new(false) as Arc<dyn Any + Send + Sync>,
    ),
    ("Integer 0", Arc::new(0i32) as Arc<dyn Any + Send + Sync>),
    ("Integer 5", Arc::new(5i32) as Arc<dyn Any + Send + Sync>),
    (
      "Empty string",
      Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Non-empty string",
      Arc::new("hello".to_string()) as Arc<dyn Any + Send + Sync>,
    ),
  ];

  for (description, value) in &test_cases {
    println!("  Converting: {}", description);
    input_tx.send(value.clone()).await.unwrap();
  }

  println!("✓ All test cases sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with ToBooleanNode...");
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
      if let Ok(arc_bool) = item.downcast::<bool>() {
        output_results.push(*arc_bool);
        println!("  Output: {}", *arc_bool);
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
    "✓ Received {} boolean conversions via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected boolean values
  let expected_results = vec![
    true,  // Boolean true
    false, // Boolean false
    false, // Integer 0
    true,  // Integer 5
    false, // Empty string
    true,  // Non-empty string
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ ToBooleanNode correctly converted all values");
    println!("  Examples:");
    for (i, &result) in output_results.iter().enumerate() {
      let (description, _) = &test_cases[i];
      println!("    {} -> {}", description, result);
    }

    println!("  Truthiness rules:");
    println!("    - Booleans: returned as-is");
    println!("    - Numbers: 0 → false, non-zero → true");
    println!("    - Strings: empty → false, non-empty → true");
  } else {
    println!(
      "⚠ ToBooleanNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
