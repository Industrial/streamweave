use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::ToIntNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("to_int_example".to_string());
  graph.add_node(
    "to_int".to_string(),
    Box::new(ToIntNode::new("to_int".to_string())),
  )?;
  graph.expose_input_port("to_int", "configuration", "configuration")?;
  graph.expose_input_port("to_int", "in", "input")?;
  graph.expose_output_port("to_int", "out", "output")?;
  graph.expose_output_port("to_int", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ToIntNode using Graph API");

  // Send configuration (optional for ToIntNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to convert to integers
  println!("📥 Sending various values for integer conversion");
  let test_cases = vec![
    ("Integer 42", Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
    ("Float 3.14", Arc::new(3.14f32) as Arc<dyn Any + Send + Sync>),
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    ("Boolean false", Arc::new(false) as Arc<dyn Any + Send + Sync>),
    ("String '42'", Arc::new("42".to_string()) as Arc<dyn Any + Send + Sync>),
    ("Invalid string", Arc::new("not_a_number".to_string()) as Arc<dyn Any + Send + Sync>),
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
  println!("Executing graph with ToIntNode...");
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
      if let Ok(arc_int) = item.downcast::<i64>() {
        output_results.push(*arc_int);
        println!("  Output: {}", *arc_int);
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
    "✓ Received {} integer conversions via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected integer values
  let expected_results = vec![
    42i64, // Integer 42
    3i64,  // Float 3.14 (truncated)
    1i64,  // Boolean true
    0i64,  // Boolean false
    42i64, // String '42'
    // Invalid string should produce error, not output
  ];

  if output_results == expected_results && error_count == 1 {
    println!("✓ ToIntNode correctly converted all values");
    println!("  Examples:");
    let descriptions = vec![
      "Integer 42 -> 42",
      "Float 3.14 -> 3 (truncated)",
      "Boolean true -> 1",
      "Boolean false -> 0",
      "String '42' -> 42",
    ];

    for (i, &result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Conversion rules:");
    println!("    - Integers: converted to i64");
    println!("    - Floats: truncated to i64");
    println!("    - Booleans: true → 1, false → 0");
    println!("    - Valid strings: parsed to i64");
    println!("    - Invalid inputs: produce errors");
  } else {
    println!(
      "⚠ ToIntNode behavior may be unexpected (received: {:?}, errors: {})",
      output_results, error_count
    );
  }

  Ok(())
}