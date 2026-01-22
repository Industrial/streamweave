use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::time::ParseTimeNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (format_tx, format_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("parse_time_example".to_string());
  graph.add_node(
    "parse_time".to_string(),
    Box::new(ParseTimeNode::new("parse_time".to_string())),
  )?;
  graph.expose_input_port("parse_time", "configuration", "configuration")?;
  graph.expose_input_port("parse_time", "in", "input")?;
  graph.expose_input_port("parse_time", "format", "format")?;
  graph.expose_output_port("parse_time", "out", "output")?;
  graph.expose_output_port("parse_time", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("format", format_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ParseTimeNode using Graph API");

  // Send configuration (optional for ParseTimeNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test different time formats
  println!("📥 Sending time strings and format specifications");

  // Test 1: RFC3339 format (auto-detected)
  println!("  Parsing: '2024-01-15T09:50:45.123Z' (RFC3339 format)");
  input_tx
    .send(Arc::new("2024-01-15T09:50:45.123Z".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Test 2: Custom format
  println!("  Parsing: '2024-01-15 09:50:45' (Custom format '%Y-%m-%d %H:%M:%S')");
  format_tx
    .send(Arc::new("%Y-%m-%d %H:%M:%S".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  input_tx
    .send(Arc::new("2024-01-15 09:50:45".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Test 3: Another custom format
  println!("  Parsing: '2024/01/15 09:50:45' (Custom format '%Y/%m/%d %H:%M:%S')");
  format_tx
    .send(Arc::new("%Y/%m/%d %H:%M:%S".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  input_tx
    .send(Arc::new("2024/01/15 09:50:45".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Test 4: Date only
  println!("  Parsing: '2024-01-15' (Date format '%Y-%m-%d')");
  format_tx
    .send(Arc::new("%Y-%m-%d".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  input_tx
    .send(Arc::new("2024-01-15".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  println!("✓ All test cases sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(format_tx);

  // Execute the graph
  println!("Executing graph with ParseTimeNode...");
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
      if let Ok(arc_timestamp) = item.downcast::<i64>() {
        output_results.push(*arc_timestamp);
        println!("  Output: timestamp = {}", *arc_timestamp);
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
    "✓ Received {} parsed timestamps via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive parsed timestamps
  if output_results.len() == 4 && error_count == 0 {
    println!("✓ ParseTimeNode correctly parsed time strings");
    println!("  Examples:");
    println!(
      "    '2024-01-15T09:50:45.123Z' with auto-detection -> timestamp {}",
      output_results[0]
    );
    println!(
      "    '2024-01-15 09:50:45' with format '%Y-%m-%d %H:%M:%S' -> timestamp {}",
      output_results[1]
    );
    println!(
      "    '2024/01/15 09:50:45' with format '%Y/%m/%d %H:%M:%S' -> timestamp {}",
      output_results[2]
    );
    println!(
      "    '2024-01-15' with format '%Y-%m-%d' -> timestamp {}",
      output_results[3]
    );

    // Verify reasonable timestamp ranges (should be in 2024)
    for &timestamp in &output_results {
      assert!(timestamp > 1700000000000i64); // After 2023
      assert!(timestamp < 1800000000000i64); // Before 2027
    }
  } else {
    println!(
      "⚠ ParseTimeNode behavior may be unexpected (received: {} timestamps, expected: 4, errors: {})",
      output_results.len(),
      error_count
    );
  }

  Ok(())
}
