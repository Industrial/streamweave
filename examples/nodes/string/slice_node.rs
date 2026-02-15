use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringSliceNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (start_tx, start_rx) = mpsc::channel(10);
  let (end_tx, end_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/string/slice_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("StringSliceNode", |id, _inputs, _outputs| {
      Box::new(StringSliceNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("start", start_rx)?;
  graph.connect_input_channel("end", end_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringSliceNode using graph! macro");

  // Send configuration (optional for StringSliceNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings, start indices, and end indices to slice
  println!("📥 Sending strings, start indices, and end indices to slice");
  let test_input_strings = [
    "Hello World".to_string(),
    "Rust Programming".to_string(),
    "Test String".to_string(),
    "Unicode: 🚀".to_string(),
    "Single".to_string(),
    "Empty".to_string(),
    "Bounds".to_string(),
  ];
  let test_start_indices = [0, 5, 0, 9, 0, 0, 3];
  let test_end_indices = [5, 16, 4, 10, 6, 0, 6];

  for i in 0..test_input_strings.len() {
    let input_str = &test_input_strings[i];
    let start_idx = test_start_indices[i];
    let end_idx = test_end_indices[i];

    // Compute expected result using character-level slicing (Unicode-safe)
    let expected = if start_idx <= end_idx {
      let char_count = input_str.chars().count();
      if end_idx <= char_count {
        // Convert character indices to byte positions
        let mut start_byte = 0;
        let mut end_byte = input_str.len();

        for (char_idx, (byte_idx, _)) in input_str.char_indices().enumerate() {
          if char_idx == start_idx {
            start_byte = byte_idx;
          }
          if char_idx == end_idx {
            end_byte = byte_idx;
            break;
          }
        }

        if end_idx == char_count {
          end_byte = input_str.len();
        }

        &input_str[start_byte..end_byte]
      } else {
        "<invalid range>"
      }
    } else {
      "<invalid range>"
    };

    println!(
      "  Slicing '{}' from {} to {} -> expected '{}'",
      input_str, start_idx, end_idx, expected
    );

    input_tx
      .send(Arc::new(input_str.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    start_tx
      .send(Arc::new(start_idx) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    end_tx
      .send(Arc::new(end_idx) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(start_tx);
  drop(end_tx);

  // Execute the graph
  println!("Executing graph with StringSliceNode...");
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

  // Verify behavior: should receive expected substring results
  let expected_results = vec![
    "Hello".to_string(),
    "Programming".to_string(),
    "Test".to_string(),
    "🚀".to_string(),
    "Single".to_string(),
    "".to_string(),
    "nds".to_string(), // "Bounds"[3..6] = "nds"
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringSliceNode correctly extracted substrings");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}'[{}-{}] -> '{}'",
        test_input_strings[i], test_start_indices[i], test_end_indices[i], result
      );
    }
  } else {
    println!(
      "⚠ StringSliceNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
