use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringConcatNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input1_tx, input1_rx) = mpsc::channel(10);
  let (input2_tx, input2_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    concat: StringConcatNode::new("concat".to_string()),
    graph.configuration => concat.configuration,
    graph.input1 => concat.in1,
    graph.input2 => concat.in2,
    concat.out => graph.output,
    concat.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input1", input1_rx)?;
  graph.connect_input_channel("input2", input2_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringConcatNode using graph! macro");

  // Send configuration (optional for StringConcatNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of strings to concatenate
  println!("📥 Sending pairs of strings to concatenate");
  let test_str1 = vec![
    "Hello".to_string(),
    "Rust".to_string(),
    "Test".to_string(),
    "".to_string(),
    "prefix".to_string(),
    "🚀".to_string(),
    "Mix".to_string(),
    "Unicode: ".to_string(),
  ];
  let test_str2 = vec![
    " World".to_string(),
    " Programming".to_string(),
    "Case".to_string(),
    "empty".to_string(),
    "".to_string(),
    "🚁".to_string(),
    "123".to_string(),
    "ñandú".to_string(),
  ];

  for i in 0..test_str1.len() {
    let str1 = &test_str1[i];
    let str2 = &test_str2[i];

    println!(
      "  Concatenating '{}' + '{}' -> expected '{}'",
      str1,
      str2,
      format!("{}{}", str1, str2)
    );

    input1_tx
      .send(Arc::new(str1.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    input2_tx
      .send(Arc::new(str2.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input1_tx);
  drop(input2_tx);

  // Execute the graph
  println!("Executing graph with StringConcatNode...");
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

  // Verify behavior: should receive expected concatenated results
  let expected_results = vec![
    "Hello World".to_string(),
    "Rust Programming".to_string(),
    "TestCase".to_string(),
    "empty".to_string(),
    "prefix".to_string(),
    "🚀🚁".to_string(),
    "Mix123".to_string(),
    "Unicode: ñandú".to_string(),
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringConcatNode correctly concatenated string pairs");
    println!("  Examples:");
    for (i, result) in output_results.iter().enumerate() {
      println!(
        "    '{}' + '{}' -> '{}'",
        test_str1[i], test_str2[i], result
      );
    }
  } else {
    println!(
      "⚠ StringConcatNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
