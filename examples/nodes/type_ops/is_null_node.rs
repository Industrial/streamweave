use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::IsNullNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("is_null_example".to_string());
  graph.add_node(
    "is_null".to_string(),
    Box::new(IsNullNode::new("is_null".to_string())),
  )?;
  graph.expose_input_port("is_null", "configuration", "configuration")?;
  graph.expose_input_port("is_null", "in", "input")?;
  graph.expose_output_port("is_null", "out", "output")?;
  graph.expose_output_port("is_null", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with IsNullNode using Graph API");

  // Send configuration (optional for IsNullNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to check if they are null
  println!("📥 Sending various values to check if they are null");
  let test_cases = vec![
    (
      "Null value (None)",
      Arc::new(None::<()>) as Arc<dyn Any + Send + Sync>,
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
    (
      "Empty object",
      Arc::new(std::collections::HashMap::<
        String,
        Arc<dyn Any + Send + Sync>,
      >::new()) as Arc<dyn Any + Send + Sync>,
    ),
    ("Object with values", {
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
  println!("Executing graph with IsNullNode...");
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
    true,  // Null value (None)
    false, // Integer value
    false, // Float value
    false, // String value
    false, // Boolean true
    false, // Boolean false
    false, // Empty string
    false, // Empty array
    false, // Array with values
    false, // Empty object
    false, // Object with values
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ IsNullNode correctly identified null values");
    println!("  Examples:");
    let descriptions = vec![
      "Null value (None) -> true (is null)",
      "Integer 42 -> false (not null)",
      "Float 3.14 -> false (not null)",
      "String 'hello world' -> false (not null)",
      "Boolean true -> false (not null)",
      "Boolean false -> false (not null)",
      "Empty string '' -> false (not null)",
      "Empty array [] -> false (not null)",
      "Array ['apple', 42] -> false (not null)",
      "Empty object {{}} -> false (not null)",
      "Object {{'name': 'John', 'age': 30}} -> false (not null)",
    ];

    for (i, &_result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Null checking rules:");
    println!("    - Null/None values: true");
    println!("    - All other values (including empty strings, arrays, objects): false");
    println!("    - No errors are generated for any input type");
    println!("  Note: In StreamWeave, null is represented as Option::None wrapped in Arc");
  } else {
    println!(
      "⚠ IsNullNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
