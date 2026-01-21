use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayIndexOfNode;
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
  let mut graph = Graph::new("index_of_example".to_string());
  graph.add_node(
    "index_of".to_string(),
    Box::new(ArrayIndexOfNode::new("index_of".to_string())),
  )?;
  graph.expose_input_port("index_of", "configuration", "configuration")?;
  graph.expose_input_port("index_of", "in", "input")?;
  graph.expose_input_port("index_of", "value", "value")?;
  graph.expose_output_port("index_of", "out", "output")?;
  graph.expose_output_port("index_of", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("value", value_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayIndexOfNode using Graph API");

  // Send configuration (optional for ArrayIndexOfNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test array: ["apple", "banana", "cherry", "date"]
  let test_array: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new("apple".to_string()),
    Arc::new("banana".to_string()),
    Arc::new("cherry".to_string()),
    Arc::new("date".to_string()),
  ];

  // Send test data: array and values to search for
  let search_values = vec!["banana", "grape", "date", "apple"]; // Found at: 1, -1, 3, 0
  for search_value in search_values {
    let _ = in_tx
      .send(Arc::new(test_array.clone()) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = value_tx
      .send(Arc::new(search_value.to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayIndexOfNode...");
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
      // ArrayIndexOfNode outputs the index as i32 (-1 if not found)
      if let Ok(index_arc) = item.clone().downcast::<i32>() {
        let index = *index_arc;
        println!("  Index: {}", index);
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

  println!("✓ Received {} successful results via output channel", success_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 4 results (indices: 1, -1, 3, 0)
  if success_count == 4 && error_count == 0 {
    println!("✓ ArrayIndexOfNode correctly found indices of values in array");
  } else {
    println!(
      "⚠ ArrayIndexOfNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 4, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
