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

  // Build the graph using the Graph API
  let mut graph = Graph::new("slice_example".to_string());
  graph.add_node(
    "slice".to_string(),
    Box::new(StringSliceNode::new("slice".to_string())),
  )?;
  graph.expose_input_port("slice", "configuration", "configuration")?;
  graph.expose_input_port("slice", "in", "input")?;
  graph.expose_input_port("slice", "start", "start")?;
  graph.expose_input_port("slice", "end", "end")?;
  graph.expose_output_port("slice", "out", "output")?;
  graph.expose_output_port("slice", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("start", start_rx)?;
  graph.connect_input_channel("end", end_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringSliceNode using Graph API");

  // Send configuration (optional for StringSliceNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings, start indices, and end indices to slice
  println!("📥 Sending strings, start indices, and end indices to slice");
  let test_input_strings = vec![
    "Hello World".to_string(),
    "Rust Programming".to_string(),
    "Test String".to_string(),
    "Unicode: 🚀".to_string(),
    "Single".to_string(),
    "Empty".to_string(),
    "Bounds".to_string(),
  ];
  let test_start_indices = vec![0, 5, 0, 9, 0, 0, 3];
  let test_end_indices = vec![5, 16, 4, 10, 6, 0, 6];

  for i in 0..test_input_strings.len() {
    let input_str = &test_input_strings[i];
    let start_idx = test_start_indices[i];
    let end_idx = test_end_indices[i];

    let expected = if start_idx <= end_idx && end_idx <= input_str.len() {
      &input_str[start_idx..end_idx]
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
      .send(Arc::new(start_idx as usize) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    end_tx
      .send(Arc::new(end_idx as usize) as Arc<dyn Any + Send + Sync>)
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

    if let Ok(Some(item)) = output_result {
      if let Ok(result_str) = item.downcast::<String>() {
        output_results.push((*result_str).clone());
        println!("  Output: '{}'", *result_str);
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
    "unds".to_string(),
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
