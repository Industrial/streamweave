use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayIndexNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (index_tx, index_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("index_example".to_string());
  graph.add_node(
    "index".to_string(),
    Box::new(ArrayIndexNode::new("index".to_string())),
  )?;
  graph.expose_input_port("index", "configuration", "configuration")?;
  graph.expose_input_port("index", "in", "input")?;
  graph.expose_input_port("index", "index", "index")?;
  graph.expose_output_port("index", "out", "output")?;
  graph.expose_output_port("index", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("index", index_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayIndexNode using Graph API");

  // Send configuration (optional for ArrayIndexNode)
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

  // Send test data: array and indices to access
  let test_indices = vec![0i32, 2i32, 3i32, 1i32]; // Valid indices: 0, 2, 3, 1
  for index in test_indices {
    let _ = in_tx
      .send(Arc::new(test_array.clone()) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = index_tx
      .send(Arc::new(index) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayIndexNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(index_tx);

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
      // ArrayIndexNode outputs the element at the specified index
      if let Ok(element_arc) = item.clone().downcast::<String>() {
        let element = (**element_arc).to_string();
        println!("  Element: {}", element);
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

  // Verify behavior: should receive 4 results (elements at indices 0, 2, 3, 1)
  if success_count == 4 && error_count == 0 {
    println!("✓ ArrayIndexNode correctly accessed array elements by index");
  } else {
    println!(
      "⚠ ArrayIndexNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 4, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
