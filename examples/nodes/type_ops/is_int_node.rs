use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::IsIntNode;
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
    is_int: IsIntNode::new("is_int".to_string()),
    graph.configuration => is_int.configuration,
    graph.input => is_int.in,
    is_int.out => graph.output,
    is_int.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with IsIntNode using graph! macro");

  // Send configuration (optional for IsIntNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to check if they are integers
  println!("📥 Sending various values to check if they are integers");
  let test_cases = vec![
    ("Integer i32", Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
    (
      "Integer i64",
      Arc::new(123456789i64) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Unsigned u32",
      Arc::new(99u32) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Unsigned u64",
      Arc::new(987654321u64) as Arc<dyn Any + Send + Sync>,
    ),
    ("Usize", Arc::new(42usize) as Arc<dyn Any + Send + Sync>),
    ("Zero", Arc::new(0i32) as Arc<dyn Any + Send + Sync>),
    (
      "Negative integer",
      Arc::new(-42i64) as Arc<dyn Any + Send + Sync>,
    ),
    ("Float f32", Arc::new(3.14f32) as Arc<dyn Any + Send + Sync>),
    (
      "Float f64",
      Arc::new(2.71828f64) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "String value",
      Arc::new("hello world".to_string()) as Arc<dyn Any + Send + Sync>,
    ),
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    (
      "Boolean false",
      Arc::new(false) as Arc<dyn Any + Send + Sync>,
    ),
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
        "score".to_string(),
        Arc::new(95.5f32) as Arc<dyn Any + Send + Sync>,
      );
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    }),
  ];

  for (description, value) in &test_cases {
    println!("  Checking: {}", description);
    input_tx.send(value.clone()).await.unwrap();
  }

  println!("✓ All test cases sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with IsIntNode...");
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
    "✓ Received {} boolean results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected boolean values
  let expected_results = vec![
    true,  // Integer i32
    true,  // Integer i64
    true,  // Unsigned u32
    true,  // Unsigned u64
    true,  // Usize
    true,  // Zero
    true,  // Negative integer
    false, // Float f32
    false, // Float f64
    false, // String value
    false, // Boolean true
    false, // Boolean false
    false, // Empty string
    false, // Empty array
    false, // Array with values
    false, // Object (HashMap)
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ IsIntNode correctly identified integer types");
    println!("  Examples:");
    let descriptions = vec![
      "Integer i32 42 -> true (is an integer)",
      "Integer i64 123456789 -> true (is an integer)",
      "Unsigned u32 99 -> true (is an integer)",
      "Unsigned u64 987654321 -> true (is an integer)",
      "Usize 42 -> true (is an integer)",
      "Zero 0 -> true (is an integer)",
      "Negative integer -42 -> true (is an integer)",
      "Float f32 3.14 -> false (not an integer)",
      "Float f64 2.71828 -> false (not an integer)",
      "String 'hello world' -> false (not an integer)",
      "Boolean true -> false (not an integer)",
      "Boolean false -> false (not an integer)",
      "Empty string '' -> false (not an integer)",
      "Empty array [] -> false (not an integer)",
      "Array ['apple', 42] -> false (not an integer)",
      "Object {{'name': 'John', 'age': 30, 'score': 95.5}} -> false (not an integer)",
    ];

    for (i, &_result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Type checking rules:");
    println!("    - Integer values (i32, i64, u32, u64, usize): true");
    println!("    - All other types (floats, strings, booleans, arrays, objects, etc.): false");
    println!("    - No errors are generated for any input type");
  } else {
    println!(
      "⚠ IsIntNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
