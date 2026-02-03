use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::stream::SampleNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (rate_tx, rate_rx) = mpsc::channel(1);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(20);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    sample: SampleNode::new("sample".to_string()),
    graph.configuration => sample.configuration,
    graph.input => sample.in,
    graph.rate => sample.rate,
    sample.out => graph.output,
    sample.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("rate", rate_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with SampleNode using graph! macro");

  // Send configuration (optional for SampleNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send sampling rate: sample every 2nd item
  println!("📥 Setting sample rate: 2 (every 2nd item)");
  let _ = rate_tx
    .send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send data to input stream
  println!("📥 Sending values to input stream: 1, 2, 3, 4, 5, 6, 7, 8");
  let input_values = vec![1i32, 2i32, 3i32, 4i32, 5i32, 6i32, 7i32, 8i32];
  for value in input_values {
    in_tx
      .send(Arc::new(value) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await; // Small delay between sends
  }

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with SampleNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(rate_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result {
      if let Ok(value) = item.downcast::<i32>() {
        let num = *value;
        output_items.push(num);
        println!("  Received sampled value: {}", num);
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
    "✓ Received {} sampled items via output channel",
    output_items.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: with rate=2, should receive items 2, 4, 6, 8 (every 2nd item)
  let expected = vec![2, 4, 6, 8];
  if output_items.len() == 4 && output_items == expected && error_count == 0 {
    println!("✓ SampleNode correctly sampled every 2nd item");
    println!("  Sampled items: {:?}", output_items);
  } else {
    println!(
      "⚠ SampleNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_items, expected, error_count
    );
  }

  Ok(())
}
