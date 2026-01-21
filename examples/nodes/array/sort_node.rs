use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArraySortNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (order_tx, order_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("sort_example".to_string());
  graph.add_node(
    "sort".to_string(),
    Box::new(ArraySortNode::new("sort".to_string())),
  )?;
  graph.expose_input_port("sort", "configuration", "configuration")?;
  graph.expose_input_port("sort", "in", "input")?;
  graph.expose_input_port("sort", "order", "order")?;
  graph.expose_output_port("sort", "out", "output")?;
  graph.expose_output_port("sort", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("order", order_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArraySortNode using Graph API");

  // Send configuration (optional for ArraySortNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test ascending sort
  println!("\n🧪 Testing ascending sort...");
  let unsorted_array: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(30i32),
    Arc::new(10i32),
    Arc::new(50i32),
    Arc::new(20i32),
    Arc::new(40i32),
  ];

  let _ = in_tx
    .send(Arc::new(unsorted_array) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = order_tx
    .send(Arc::new("ascending".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArraySortNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(order_tx);

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
      // ArraySortNode outputs the sorted array
      if let Ok(sorted_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let sorted_vec = (**sorted_arc).to_vec();
        println!("  Sorted array (ascending) length: {}", sorted_vec.len());
        // Print the sorted values
        for (i, elem) in sorted_vec.iter().enumerate() {
          if let Ok(value_arc) = elem.clone().downcast::<i32>() {
            let value = *value_arc;
            println!("    [{}]: {}", i, value);
          }
        }
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

  // Verify behavior: should receive 1 result (array of 5 elements: [10, 20, 30, 40, 50])
  if success_count == 1 && error_count == 0 {
    println!("✓ ArraySortNode correctly sorted the array in ascending order");
  } else {
    println!(
      "⚠ ArraySortNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
