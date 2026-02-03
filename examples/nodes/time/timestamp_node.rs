use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::time::TimestampNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    timestamp: TimestampNode::new("timestamp".to_string()),
    graph.configuration => timestamp.configuration,
    graph.input => timestamp.in,
    timestamp.out => graph.output,
    timestamp.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with TimestampNode using graph! macro");

  // Send configuration (optional for TimestampNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: different types of items to timestamp
  println!("📥 Sending various items to timestamp");
  let test_items = vec![
    Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>,
    Arc::new(42i64) as Arc<dyn Any + Send + Sync>,
    Arc::new(true) as Arc<dyn Any + Send + Sync>,
    {
      let mut obj = HashMap::new();
      obj.insert(
        "name".to_string(),
        Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>,
      );
      obj.insert(
        "age".to_string(),
        Arc::new(30i64) as Arc<dyn Any + Send + Sync>,
      );
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    },
  ];

  for (i, item) in test_items.into_iter().enumerate() {
    println!("  Sending item {}: {:?}", i + 1, item);
    input_tx.send(item).await.unwrap();

    // Small delay between items
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  println!("✓ All test items sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with TimestampNode...");
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
      output_results.push(item);
      println!("  Output: timestamped item received");
      has_data = true;
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
    "✓ Received {} timestamped items via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive timestamped items
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_millis() as i64;

  println!("\n📊 Timestamp Analysis:");
  println!("Current time: {}", now);

  if output_results.len() == 4 && error_count == 0 {
    println!("✓ TimestampNode correctly added timestamps to items");

    for (i, item) in output_results.iter().enumerate() {
      println!("  Item {}:", i + 1);

      // Try to downcast as HashMap (object with timestamp)
      if let Ok(arc_map) = item
        .clone()
        .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
      {
        let map = &*arc_map;
        println!("    Object with {} properties:", map.len());

        for (key, value) in map {
          if key == "timestamp" {
            if let Ok(arc_timestamp) = value.clone().downcast::<i64>() {
              let timestamp = *arc_timestamp;
              let diff = (now - timestamp).abs();
              println!(
                "      {}: {} (diff: {}ms from current time)",
                key, timestamp, diff
              );
              assert!(
                diff < 5000,
                "Timestamp should be within 5 seconds of current time"
              );
            }
          } else if key == "value" {
            println!("      {}: {:?}", key, value);
          } else {
            println!("      {}: {:?}", key, value);
          }
        }
      } else {
        println!("    Unexpected item type: {:?}", item);
      }
    }

    println!("  ✓ All items properly timestamped");
  } else {
    println!(
      "⚠ TimestampNode behavior may be unexpected (received: {} items, expected: 4, errors: {})",
      output_results.len(),
      error_count
    );
  }

  Ok(())
}
