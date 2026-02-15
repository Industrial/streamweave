use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringPrependNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (prefix_tx, prefix_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/string/prepend_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("StringPrependNode", |id, _inputs, _outputs| {
      Box::new(StringPrependNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("prefix", prefix_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringPrependNode using graph! macro");

  // Send configuration (optional for StringPrependNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: base strings and prefixes to prepend
  println!("📥 Sending base strings and prefixes to prepend");
  let test_bases = [
    "World".to_string(),
    "42".to_string(),
    ".txt".to_string(),
    " 1\n".to_string(),
  ];
  let test_prefixes = [
    "Hello ".to_string(),
    "Count: ".to_string(),
    "file".to_string(),
    "Line".to_string(),
  ];

  for i in 0..test_bases.len() {
    println!(
      "  Prepending: '{}' + '{}' -> expected '{}{}'",
      test_prefixes[i], test_bases[i], test_prefixes[i], test_bases[i]
    );

    input_tx
      .send(Arc::new(test_bases[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    prefix_tx
      .send(Arc::new(test_prefixes[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(prefix_tx);

  // Execute the graph
  println!("Executing graph with StringPrependNode...");
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

  // Verify behavior: should receive expected prepended results
  let expected_results = vec![
    "Hello World".to_string(),
    "Count: 42".to_string(),
    "file.txt".to_string(),
    "Line 1\n".to_string(),
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringPrependNode correctly prepended prefixes to base strings");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}' + '{}' -> '{}'",
        test_prefixes[i], test_bases[i], result
      );
    }
  } else {
    println!(
      "⚠ StringPrependNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
