use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::IsArrayNode;
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
    is_array: IsArrayNode::new("is_array".to_string()),
    graph.configuration => is_array.configuration,
    graph.input => is_array.in,
    is_array.out => graph.output,
    is_array.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with IsArrayNode using graph! macro");

  // Send configuration (optional for IsArrayNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to check if they are arrays
  println!("📥 Sending various values to check if they are arrays");
  let test_cases = vec![
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
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    (
      "Boolean false",
      Arc::new(false) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Empty array",
      Arc::new(Vec::<Arc<dyn Any + Send + Sync>>::new()) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Array with strings",
      Arc::new(vec![
        Arc::new("apple".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("banana".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("cherry".to_string()) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Array with numbers",
      Arc::new(vec![
        Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Array with mixed types",
      Arc::new(vec![
        Arc::new("text".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new(42i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(true) as Arc<dyn Any + Send + Sync>,
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
  println!("Executing graph with IsArrayNode...");
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
    "✓ Received {} boolean results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected boolean values
  let expected_results = vec![
    false, // String value
    false, // Integer value
    false, // Float value
    false, // Boolean true
    false, // Boolean false
    true,  // Empty array
    true,  // Array with strings
    true,  // Array with numbers
    true,  // Array with mixed types
    false, // Object (HashMap)
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ IsArrayNode correctly identified array types");
    println!("  Examples:");
    let descriptions = vec![
      "String 'hello world' -> false (not an array)",
      "Integer 42 -> false (not an array)",
      "Float 3.14 -> false (not an array)",
      "Boolean true -> false (not an array)",
      "Boolean false -> false (not an array)",
      "Empty array [] -> true (is an array)",
      "Array ['apple', 'banana', 'cherry'] -> true (is an array)",
      "Array [1, 2, 3] -> true (is an array)",
      "Array ['text', 42, true] -> true (is an array)",
      "Object {{'name': 'John', 'age': 30}} -> false (not an array)",
    ];

    for (i, &_result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Type checking rules:");
    println!("    - Arrays (Vec<Arc<dyn Any + Send + Sync>>): true");
    println!("    - All other types (String, numbers, bool, HashMap, etc.): false");
    println!("    - No errors are generated for any input type");
  } else {
    println!(
      "⚠ IsArrayNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
