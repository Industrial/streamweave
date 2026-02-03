use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringAppendNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (base_tx, base_rx) = mpsc::channel(10);
  let (suffix_tx, suffix_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    append: StringAppendNode::new("append".to_string()),
    graph.configuration => append.configuration,
    graph.base => append.in,
    graph.suffix => append.suffix,
    append.out => graph.output,
    append.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("base", base_rx)?;
  graph.connect_input_channel("suffix", suffix_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringAppendNode using graph! macro");

  // Send configuration (optional for StringAppendNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: base strings and suffixes to append
  println!("📥 Sending base strings and suffixes to append");
  let test_bases = vec![
    "Hello".to_string(),
    "Count: ".to_string(),
    "File".to_string(),
    "Line".to_string(),
  ];
  let test_suffixes = vec![
    " World".to_string(),
    "42".to_string(),
    ".txt".to_string(),
    " 1\n".to_string(),
  ];

  for i in 0..test_bases.len() {
    base_tx
      .send(Arc::new(test_bases[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    suffix_tx
      .send(Arc::new(test_suffixes[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    println!(
      "  Sent: '{}' + '{}' -> expected '{}'",
      test_bases[i],
      test_suffixes[i],
      format!("{}{}", test_bases[i], test_suffixes[i])
    );
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(base_tx);
  drop(suffix_tx);

  // Execute the graph
  println!("Executing graph with StringAppendNode...");
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

  // Verify behavior: should receive expected appended results
  let expected_results = vec![
    "Hello World".to_string(),
    "Count: 42".to_string(),
    "File.txt".to_string(),
    "Line 1\n".to_string(),
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringAppendNode correctly appended suffixes to base strings");
  } else {
    println!(
      "⚠ StringAppendNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
