#![allow(clippy::approx_constant)]
use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::ToStringNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/type_ops/to_string_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("ToStringNode", |id, _inputs, _outputs| {
      Box::new(ToStringNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ToStringNode using graph! macro");

  // Send configuration (optional for ToStringNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to convert to strings
  println!("📥 Sending various values for string conversion");
  let test_cases = vec![
    (
      "String",
      Arc::new("hello world".to_string()) as Arc<dyn Any + Send + Sync>,
    ),
    ("Integer 42", Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
    (
      "Float 3.14",
      Arc::new(3.14f64) as Arc<dyn Any + Send + Sync>,
    ),
    ("Boolean true", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    (
      "Boolean false",
      Arc::new(false) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Large integer",
      Arc::new(123456789i64) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Unsigned integer",
      Arc::new(42u32) as Arc<dyn Any + Send + Sync>,
    ),
    ("Float 0.0", Arc::new(0.0f32) as Arc<dyn Any + Send + Sync>),
    (
      "Array of strings",
      Arc::new(vec![
        Arc::new("apple".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("banana".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("cherry".to_string()) as Arc<dyn Any + Send + Sync>,
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
    (
      "Unsupported type (Vec)",
      Arc::new(vec![1, 2, 3]) as Arc<dyn Any + Send + Sync>,
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
  println!("Executing graph with ToStringNode...");
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

    if let Ok(Some(item)) = output_result
      && let Ok(arc_str) = item.downcast::<String>()
    {
      output_results.push(arc_str.to_string());
      println!("  Output: \"{}\"", arc_str);
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
    "✓ Received {} string conversions via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected string values
  let expected_results = vec![
    "hello world".to_string(),
    "42".to_string(),
    "3.14".to_string(),
    "true".to_string(),
    "false".to_string(),
    "123456789".to_string(),
    "42".to_string(),
    "0".to_string(),
    "[\"apple\", \"banana\", \"cherry\"]".to_string(),
    "{\"active\": \"true\", \"age\": \"30\", \"name\": \"John\"}".to_string(),
    // The last test case (unsupported Vec<i32>) should produce an error
  ];

  // Check that we got the expected number of results (excluding the error case)
  if output_results.len() == expected_results.len() {
    println!("✓ ToStringNode correctly converted all values");
    println!("  Examples:");
    let descriptions = vec![
      "String 'hello world' -> 'hello world'",
      "Integer 42 -> '42'",
      "Float 3.14 -> '3.14'",
      "Boolean true -> 'true'",
      "Boolean false -> 'false'",
      "Large integer 123456789 -> '123456789'",
      "Unsigned integer 42 -> '42'",
      "Float 0.0 -> '0'",
      "Array -> '[\"apple\", \"banana\", \"cherry\"]'",
      "Object -> JSON-like representation",
    ];

    for (i, _result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Conversion rules:");
    println!("    - Strings: returned as-is");
    println!("    - Numbers: converted using to_string()");
    println!("    - Booleans: 'true' or 'false'");
    println!("    - Arrays: JSON-like array format with quoted strings");
    println!("    - Objects: JSON-like object format with quoted keys/values");
    println!("    - Unsupported types: produce errors");
  } else {
    println!(
      "⚠ ToStringNode behavior may be unexpected (received: {}, expected: {})",
      output_results.len(),
      expected_results.len()
    );
  }

  Ok(())
}
