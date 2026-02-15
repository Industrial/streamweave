#![allow(clippy::approx_constant)]
use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::IsFloatNode;
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
    let path = Path::new("examples/nodes/type_ops/is_float_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("IsFloatNode", |id, _inputs, _outputs| {
      Box::new(IsFloatNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with IsFloatNode using graph! macro");

  // Send configuration (optional for IsFloatNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to check if they are floats
  println!("📥 Sending various values to check if they are floats");
  let test_cases = vec![
    ("Float f32", Arc::new(3.14f32) as Arc<dyn Any + Send + Sync>),
    (
      "Float f64",
      Arc::new(2.718281828f64) as Arc<dyn Any + Send + Sync>,
    ),
    ("Zero f32", Arc::new(0.0f32) as Arc<dyn Any + Send + Sync>),
    (
      "Negative float",
      Arc::new(-1.5f64) as Arc<dyn Any + Send + Sync>,
    ),
    ("Integer i32", Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
    (
      "Integer i64",
      Arc::new(123456789i64) as Arc<dyn Any + Send + Sync>,
    ),
    (
      "Unsigned integer",
      Arc::new(42u32) as Arc<dyn Any + Send + Sync>,
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
  println!("Executing graph with IsFloatNode...");
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
    true,  // Float f32
    true,  // Float f64
    true,  // Zero f32
    true,  // Negative float
    false, // Integer i32
    false, // Integer i64
    false, // Unsigned integer
    false, // String value
    false, // Boolean true
    false, // Boolean false
    false, // Empty string
    false, // Empty array
    false, // Array with values
    false, // Object (HashMap)
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ IsFloatNode correctly identified float types");
    println!("  Examples:");
    let descriptions = vec![
      "Float f32 3.14 -> true (is a float)",
      "Float f64 2.718281828 -> true (is a float)",
      "Zero f32 0.0 -> true (is a float)",
      "Negative float -1.5 -> true (is a float)",
      "Integer i32 42 -> false (not a float)",
      "Integer i64 123456789 -> false (not a float)",
      "Unsigned integer 42 -> false (not a float)",
      "String 'hello world' -> false (not a float)",
      "Boolean true -> false (not a float)",
      "Boolean false -> false (not a float)",
      "Empty string '' -> false (not a float)",
      "Empty array [] -> false (not a float)",
      "Array ['apple', 42] -> false (not a float)",
      "Object {{'name': 'John', 'age': 30, 'score': 95.5}} -> false (not a float)",
    ];

    for (i, &_result) in output_results.iter().enumerate() {
      if i < descriptions.len() {
        println!("    {}", descriptions[i]);
      }
    }

    println!("  Type checking rules:");
    println!("    - Float values (f32, f64): true");
    println!("    - All other types (integers, strings, booleans, arrays, objects, etc.): false");
    println!("    - No errors are generated for any input type");
  } else {
    println!(
      "⚠ IsFloatNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
