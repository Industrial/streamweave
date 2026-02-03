use std::any::Any;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::time::CurrentTimeNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (trigger_tx, trigger_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    current_time: CurrentTimeNode::new("current_time".to_string()),
    graph.configuration => current_time.configuration,
    graph.trigger => current_time.trigger,
    current_time.out => graph.output,
    current_time.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("trigger", trigger_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with CurrentTimeNode using graph! macro");

  // Send configuration (optional for CurrentTimeNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send multiple trigger signals to generate timestamps
  println!("📥 Sending trigger signals to generate current timestamps");
  let num_triggers = 5;

  for i in 0..num_triggers {
    println!("  Sending trigger {}...", i + 1);
    trigger_tx
      .send(Arc::new(format!("trigger_{}", i)) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    // Small delay between triggers to ensure different timestamps
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  println!("✓ All trigger signals sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(trigger_tx);

  // Execute the graph
  println!("Executing graph with CurrentTimeNode...");
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
      && let Ok(arc_timestamp) = item.downcast::<i64>()
    {
      output_results.push(*arc_timestamp);
      println!("  Output: {} (timestamp)", *arc_timestamp);
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
    "✓ Received {} timestamps via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive timestamps close to current time
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;

  println!("\n📊 Timestamp Analysis:");
  println!("Current time: {}", now);

  if output_results.len() == num_triggers && error_count == 0 {
    println!("✓ CurrentTimeNode correctly generated current timestamps");
    println!("  Timestamps:");

    for (i, &timestamp) in output_results.iter().enumerate() {
      let diff = (now - timestamp).abs();
      println!(
        "    Trigger {}: {} (diff: {}ms from current time)",
        i + 1,
        timestamp,
        diff
      );
      assert!(
        diff < 5000,
        "Timestamp should be within 5 seconds of current time"
      );
    }

    // Verify timestamps are monotonically increasing
    println!("  Verifying timestamp ordering...");
    for i in 1..output_results.len() {
      assert!(
        output_results[i] >= output_results[i - 1],
        "Timestamp {} should be >= previous timestamp {}",
        output_results[i],
        output_results[i - 1]
      );
    }
    println!("  ✓ All timestamps are in correct order");
  } else {
    println!(
      "⚠ CurrentTimeNode behavior may be unexpected (received: {} timestamps, expected: {}, errors: {})",
      output_results.len(),
      num_triggers,
      error_count
    );
  }

  Ok(())
}
