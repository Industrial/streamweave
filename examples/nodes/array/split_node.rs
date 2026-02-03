use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArraySplitNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (chunk_size_tx, chunk_size_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    split: ArraySplitNode::new("split".to_string()),
    graph.configuration => split.configuration,
    graph.input => split.in,
    graph.chunk_size => split.chunk_size,
    split.out => graph.output,
    split.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("chunk_size", chunk_size_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArraySplitNode using graph! macro");

  // Send configuration (optional for ArraySplitNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test array splitting
  println!("\n🧪 Testing array splitting...");
  let test_array: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(10i32),
    Arc::new(20i32),
    Arc::new(30i32),
    Arc::new(40i32),
    Arc::new(50i32),
    Arc::new(60i32),
    Arc::new(70i32),
  ];

  // Split into chunks of size 3
  let _ = in_tx
    .send(Arc::new(test_array) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = chunk_size_tx
    .send(Arc::new(3i32) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArraySplitNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(chunk_size_tx);

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
      // ArraySplitNode outputs Vec<Arc<dyn Any + Send + Sync>> where each element is a chunk
      if let Ok(chunks_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let chunks = (**chunks_arc).to_vec();
        println!("  Split into {} chunks:", chunks.len());

        // Print each chunk
        for (chunk_idx, chunk_arc) in chunks.iter().enumerate() {
          if let Ok(chunk) = chunk_arc
            .clone()
            .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
          {
            let chunk_vec = (**chunk).to_vec();
            println!("    Chunk {} (size {}):", chunk_idx, chunk_vec.len());
            for (elem_idx, elem) in chunk_vec.iter().enumerate() {
              if let Ok(value_arc) = elem.clone().downcast::<i32>() {
                let value = *value_arc;
                println!("      [{}]: {}", elem_idx, value);
              }
            }
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

  // Verify behavior: should receive 1 result (3 chunks: [10,20,30], [40,50,60], [70])
  if success_count == 1 && error_count == 0 {
    println!("✓ ArraySplitNode correctly split the array into chunks");
  } else {
    println!(
      "⚠ ArraySplitNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
