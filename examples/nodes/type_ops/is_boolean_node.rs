#![allow(clippy::approx_constant)]
use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::IsBooleanNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(100);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(100);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(100);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    is_boolean: IsBooleanNode::new("is_boolean".to_string()),
    graph.configuration => is_boolean.configuration,
    graph.input => is_boolean.in,
    is_boolean.out => graph.output,
    is_boolean.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with IsBooleanNode using graph! macro");

  // Test various values to check if they are booleans
  println!("📥 Sending various values to check if they are booleans");
  let test_cases = vec![
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    (
      "Boolean false",
      Arc::new(false) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "String value",
      Arc::new("hello world".to_string()) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Integer value",
      Arc::new(42i32) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Float value",
      Arc::new(3.14f64) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Large integer",
      Arc::new(123456789i64) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Unsigned integer",
      Arc::new(42u32) as Arc<dyn Any + Send + Sync>,
    ),
    ("Zero", Arc::new(0i32) as Arc<dyn Any + Send + Sync>),
    (
      "Empty string",
      Arc::new("".to_string()) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Empty array",
      Arc::new(Vec::<Arc<dyn Any + Send + Sync>>::new()) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Array with values",
      Arc::new(vec![
        Arc::new("apple".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new(42i32) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
    ),
    ("Object (HashMap)", {
      let mut obj = std::collections::HashMap::new();
      obj.insert(
        "name".to_string(),
        Arc::new("John".to_string()) as Arc<dyn Any + Send + Sync>,
      );
      obj.insert(
        "age".to_string(),
        Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
      );
      obj.insert(
        "active".to_string(),
        Arc::new(true) as Arc<dyn Any + Send + Sync>,
      );
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    }),
  ];

  for (description, value) in &test_cases {
    println!("  Checking: {}", description);
    input_tx.send(value.clone()).await.unwrap();
    // Small delay to prevent overwhelming the channels
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
  }

  println!("✓ All test cases sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with IsBooleanNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  graph
    .wait_for_completion()
    .await
    .map_err(|e| format!("Graph wait failed: {:?}", e))?;
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

    if let Ok(Some(item)) = output_result
      && let Ok(arc_bool) = item.downcast::<bool>()
    {
      output_results.push(*arc_bool);
      println!("  Output: {}", *arc_bool);
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
    "✓ Received {} boolean results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected boolean values
  let expected_results = vec![
    true,  // Boolean true
    true,  // Boolean false
    false, // String value
    false, // Integer value
    false, // Float value
    false, // Large integer
    false, // Unsigned integer
    false, // Zero
    false, // Empty string
    false, // Empty array
    false, // Array with values
    false, // Object (HashMap)
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ IsBooleanNode correctly identified boolean types");
    println!("  Examples:");
    let descriptions = vec![
      "Boolean true -> true (is a boolean)",
      "Boolean false -> true (is a boolean)",
      "String 'hello world' -> false (not a boolean)",
      "Integer 42 -> false (not a boolean)",
      "Float 3.14 -> false (not a boolean)",
      "Large integer 123456789 -> false (not a boolean)",
      "Unsigned integer 42 -> false (not a boolean)",
      "Zero 0 -> false (not a boolean)",
      "Empty string '' -> false (not a boolean)",
      "Empty array [] -> false (not a boolean)",
      "Array ['apple', 42] -> false (not a boolean)",
      "Object {{'name': 'John', 'age': 30, 'active': true}} -> false (not a boolean)",
    ];

    for (i, &_result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Type checking rules:");
    println!("    - Boolean values (true/false): true");
    println!("    - All other types (String, numbers, arrays, objects, etc.): false");
    println!("    - No errors are generated for any input type");
  } else {
    println!(
      "⚠ IsBooleanNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
