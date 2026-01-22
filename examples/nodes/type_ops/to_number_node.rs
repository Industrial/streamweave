use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::ToNumberNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("to_number_example".to_string());
  graph.add_node(
    "to_number".to_string(),
    Box::new(ToNumberNode::new("to_number".to_string())),
  )?;
  graph.expose_input_port("to_number", "configuration", "configuration")?;
  graph.expose_input_port("to_number", "in", "input")?;
  graph.expose_output_port("to_number", "out", "output")?;
  graph.expose_output_port("to_number", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ToNumberNode using Graph API");

  // Send configuration (optional for ToNumberNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to convert to numbers
  println!("📥 Sending various values for number conversion");
  let test_cases = vec![
    ("Integer 42", Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
    ("Float 3.14", Arc::new(3.14f32) as Arc<dyn Any + Send + Sync>),
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
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
  println!("Executing graph with ToNumberNode...");
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
      if let Ok(arc_number) = item.downcast::<f64>() {
        output_results.push(*arc_number);
        println!("  Output: {}", *arc_number);
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
    "✓ Received {} number conversions via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected numeric values
  // Check results with tolerance for floating point precision
  let results_match = output_results.len() == 4 &&
    (output_results[0] - 42.0).abs() < 0.001 &&  // Integer 42
    (output_results[1] - 3.14).abs() < 0.001 &&  // Float 3.14 (allow small precision differences)
    (output_results[2] - 1.0).abs() < 0.001 &&   // Boolean true
    (output_results[3] - 42.0).abs() < 0.001;    // String '42'

  if results_match && error_count == 1 {
    println!("✓ ToNumberNode correctly converted all values");
    println!("  Examples:");
    let descriptions = vec![
      "Integer 42 -> 42.0",
      "Float 3.14 -> 3.14",
      "Boolean true -> 1.0",
      "String '42' -> 42.0",
    ];

    for (i, &result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Conversion rules:");
    println!("    - Integers: converted to f64");
    println!("    - Floats: converted to f64");
    println!("    - Booleans: true → 1.0, false → 0.0");
    println!("    - Valid strings: parsed to f64");
    println!("    - Invalid strings: produce errors");
  } else {
    println!(
      "⚠ ToNumberNode behavior may be unexpected (received: {:?}, errors: {})",
      output_results, error_count
    );
  }

  Ok(())
}