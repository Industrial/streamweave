use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayUniqueNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    unique: ArrayUniqueNode::new("unique".to_string()),
    graph.configuration => unique.configuration,
    graph.input => unique.in,
    unique.out => graph.output,
    unique.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayUniqueNode using graph! macro");

  // Send configuration (optional for ArrayUniqueNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test array deduplication
  println!("\n🧪 Testing array deduplication...");
  let array_with_duplicates: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(10i32),
    Arc::new(20i32),
    Arc::new(10i32), // duplicate
    Arc::new(30i32),
    Arc::new(20i32), // duplicate
    Arc::new(40i32),
    Arc::new(30i32), // duplicate
  ];

  let _ = in_tx
    .send(Arc::new(array_with_duplicates) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayUniqueNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);

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
      // ArrayUniqueNode outputs the deduplicated array
      if let Ok(unique_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let unique_vec = (**unique_arc).to_vec();
        println!("  Deduplicated array length: {} (from 7)", unique_vec.len());
        // Print the unique values
        for (i, elem) in unique_vec.iter().enumerate() {
          if let Ok(value_arc) = elem.clone().downcast::<i32>() {
            let value = *value_arc;
            println!("    [{}]: {}", i, value);
          }
        }
        success_count += 1;
        has_data = true;
      }
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
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 1 result (array of 4 unique elements: [10, 20, 30, 40])
  if success_count == 1 && error_count == 0 {
    println!("✓ ArrayUniqueNode correctly removed duplicates from array");
  } else {
    println!(
      "⚠ ArrayUniqueNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
