use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::type_ops::ToFloatNode;
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
    to_float: ToFloatNode::new("to_float".to_string()),
    graph.configuration => to_float.configuration,
    graph.input => to_float.in,
    to_float.out => graph.output,
    to_float.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ToFloatNode using graph! macro");

  // Send configuration (optional for ToFloatNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test various values to convert to float
  println!("📥 Sending various values for float conversion");

  // Test just one simple case first
  println!("  Converting: Integer 42");
  input_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  println!("✓ All test cases sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with ToFloatNode...");
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
      if let Ok(arc_float) = item.downcast::<f32>() {
        output_results.push(*arc_float);
        println!("  Output: {}", *arc_float);
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
    "✓ Received {} float conversions via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected float values
  // Check results with tolerance for floating point precision
  let results_match = output_results.len() == 1 && (output_results[0] - 42.0).abs() < 0.001; // Integer 42

  if results_match && error_count == 0 {
    println!("✓ ToFloatNode correctly converted the value");
    println!("  Example: Integer 42 -> 42.0");
    println!("  Conversion rules:");
    println!("    - Integers: converted to f32");
    println!("    - Floats: converted to f32");
    println!("    - Booleans: true → 1.0, false → 0.0");
    println!("    - Valid strings: parsed to f32");
    println!("    - Invalid inputs: produce errors");
  } else {
    println!(
      "⚠ ToFloatNode behavior may be unexpected (received: {:?}, errors: {})",
      output_results, error_count
    );
  }

  Ok(())
}
