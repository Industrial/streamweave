use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringCharAtNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (index_tx, index_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    char_at: StringCharAtNode::new("char_at".to_string()),
    graph.configuration => char_at.configuration,
    graph.input => char_at.in,
    graph.index => char_at.index,
    char_at.out => graph.output,
    char_at.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("index", index_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringCharAtNode using graph! macro");

  // Send configuration (optional for StringCharAtNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings and indices to get characters from
  println!("📥 Sending strings and indices to get characters from");
  let test_input_strings = vec![
    "Hello".to_string(),
    "World".to_string(),
    "🚀🚁🚂".to_string(),
    "Test".to_string(),
    "Single".to_string(),
    "Unicode: ñ".to_string(),
    "Mixed123".to_string(),
  ];
  let test_indices = vec![0, 2, 1, 3, 0, 9, 5];

  for i in 0..test_input_strings.len() {
    let input_str = &test_input_strings[i];
    let index = test_indices[i];

    println!(
      "  Getting character at index {} from '{}' -> expected '{}'",
      index,
      input_str,
      input_str.chars().nth(index).unwrap_or('?')
    );

    input_tx
      .send(Arc::new(input_str.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    index_tx
      .send(Arc::new(index as usize) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(index_tx);

  // Execute the graph
  println!("Executing graph with StringCharAtNode...");
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

  // Verify behavior: should receive expected character results
  let expected_results = vec![
    "H".to_string(),
    "r".to_string(),
    "🚁".to_string(),
    "t".to_string(),
    "S".to_string(),
    "ñ".to_string(),
    "2".to_string(),
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringCharAtNode correctly extracted characters by index");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}'[{:}] -> '{}'",
        test_input_strings[i], test_indices[i], result
      );
    }
  } else {
    println!(
      "⚠ StringCharAtNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
