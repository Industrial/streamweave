use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringSplitNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (delimiter_tx, delimiter_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    split: StringSplitNode::new("split".to_string()),
    graph.configuration => split.configuration,
    graph.input => split.in,
    graph.delimiter => split.delimiter,
    split.out => graph.output,
    split.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("delimiter", delimiter_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringSplitNode using graph! macro");

  // Send configuration (optional for StringSplitNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: strings and delimiters to split by
  println!("📥 Sending strings and delimiters to split by");
  let test_input_strings = [
    "apple,banana,cherry".to_string(),
    "hello world rust".to_string(),
    "one-two-three-four".to_string(),
    "no-delimiter-here".to_string(),
    "".to_string(),
    "single".to_string(),
    "a,b,c,".to_string(),
  ];
  let test_delimiters = [
    ",".to_string(),
    " ".to_string(),
    "-".to_string(),
    "|".to_string(),
    ",".to_string(),
    ",".to_string(),
    ",".to_string(),
  ];

  for i in 0..test_input_strings.len() {
    println!(
      "  Splitting '{}' by '{}' -> expected {} parts",
      test_input_strings[i],
      test_delimiters[i],
      test_input_strings[i].split(&test_delimiters[i]).count()
    );

    input_tx
      .send(Arc::new(test_input_strings[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    delimiter_tx
      .send(Arc::new(test_delimiters[i].clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(delimiter_tx);

  // Execute the graph
  println!("Executing graph with StringSplitNode...");
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
      && let Ok(vec_arc) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    {
      let mut parts = Vec::new();
      for part in vec_arc.iter() {
        if let Ok(part_str) = part.clone().downcast::<String>() {
          parts.push((*part_str).clone());
        }
      }
      let parts_clone = parts.clone();
      output_results.push(parts);
      println!("  Output: {:?}", parts_clone);
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

  // Verify behavior: should receive expected split results
  let expected_results = vec![
    vec![
      "apple".to_string(),
      "banana".to_string(),
      "cherry".to_string(),
    ],
    vec!["hello".to_string(), "world".to_string(), "rust".to_string()],
    vec![
      "one".to_string(),
      "two".to_string(),
      "three".to_string(),
      "four".to_string(),
    ],
    vec!["no-delimiter-here".to_string()],
    Vec::<String>::new(),
    vec!["single".to_string()],
    vec![
      "a".to_string(),
      "b".to_string(),
      "c".to_string(),
      "".to_string(),
    ],
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringSplitNode correctly split strings by delimiters");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}' split by '{}' -> {:?}",
        test_input_strings[i], test_delimiters[i], result
      );
    }
  } else {
    println!(
      "⚠ StringSplitNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
