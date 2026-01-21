use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayContainsNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("contains_example".to_string());
  graph.add_node(
    "contains".to_string(),
    Box::new(ArrayContainsNode::new("contains".to_string())),
  )?;
  graph.expose_input_port("contains", "configuration", "configuration")?;
  graph.expose_input_port("contains", "in", "input")?;
  graph.expose_input_port("contains", "value", "value")?;
  graph.expose_output_port("contains", "out", "output")?;
  graph.expose_output_port("contains", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("value", value_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayContainsNode using Graph API");

  // Send configuration (optional for ArrayContainsNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: arrays and values to search for
  let test_cases = vec![
    (vec!["apple", "banana", "cherry"], "banana"), // contains: true
    (vec!["apple", "banana", "cherry"], "grape"),  // contains: false
    (vec!["x", "y", "z"], "x"),                    // contains: true
    (vec!["x", "y", "z"], "w"),                    // contains: false
  ];

  for (array_data, search_value) in test_cases {
    let array_vec: Vec<Arc<dyn Any + Send + Sync>> = array_data
      .iter()
      .map(|s| Arc::new(s.to_string()) as Arc<dyn Any + Send + Sync>)
      .collect();

    let _ = in_tx
      .send(Arc::new(array_vec) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = value_tx
      .send(Arc::new(search_value.to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayContainsNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(value_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut success_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // ArrayContainsNode outputs a boolean result
      if let Ok(contains_arc) = item.clone().downcast::<bool>() {
        let contains = *contains_arc;
        println!("  Contains: {}", contains);
        success_count += 1;
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
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 4 results (2 true, 2 false)
  if success_count == 4 && error_count == 0 {
    println!("✓ ArrayContainsNode correctly checked array containment");
  } else {
    println!(
      "⚠ ArrayContainsNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 4, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
