use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringUppercaseNode;
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
    let path = Path::new("examples/nodes/string/uppercase_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("StringUppercaseNode", |id, _inputs, _outputs| {
      Box::new(StringUppercaseNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringUppercaseNode using graph! macro");

  // Send configuration (optional for StringUppercaseNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings to convert to uppercase
  println!("📥 Sending strings to convert to uppercase");
  let test_cases = vec![
    "hello world".to_string(),
    "Rust Programming".to_string(),
    "MIXED case STRING".to_string(),
    "already uppercase".to_string(),
    "".to_string(),
    "ñandú".to_string(),
    "café".to_string(),
  ];

  for input_str in &test_cases {
    println!(
      "  Converting '{}' -> expected '{}'",
      input_str,
      input_str.to_uppercase()
    );

    input_tx
      .send(Arc::new(input_str.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with StringUppercaseNode...");
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
      && let Ok(result_str) = item.downcast::<String>()
    {
      output_results.push((*result_str).clone());
      println!("  Output: '{}'", *result_str);
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
    "✓ Received {} results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected uppercase results
  let expected_results = vec![
    "HELLO WORLD".to_string(),
    "RUST PROGRAMMING".to_string(),
    "MIXED CASE STRING".to_string(),
    "ALREADY UPPERCASE".to_string(),
    "".to_string(),
    "ÑANDÚ".to_string(),
    "CAFÉ".to_string(),
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringUppercaseNode correctly converted strings to uppercase");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!("    '{}' -> '{}'", test_cases[i], result);
    }
  } else {
    println!(
      "⚠ StringUppercaseNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
