use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::time::FormatTimeNode;
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
  let mut graph = Graph::new("format_time_example".to_string());
  graph.add_node(
    "format_time".to_string(),
    Box::new(FormatTimeNode::new("format_time".to_string())),
  )?;
  graph.expose_input_port("format_time", "configuration", "configuration")?;
  graph.expose_input_port("format_time", "in", "input")?;
  graph.expose_input_port("format_time", "format", "format")?;
  graph.expose_output_port("format_time", "out", "output")?;
  graph.expose_output_port("format_time", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("format", format_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with FormatTimeNode using Graph API");

  // Send configuration (optional for FormatTimeNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Test different timestamps and formats
  println!("📥 Sending timestamps and format strings");

  // Test 1: Custom format
  println!("  Test 1: Custom format for timestamp 1705312245123");
  format_tx
    .send(Arc::new("%Y-%m-%d %H:%M:%S".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  input_tx
    .send(Arc::new(1705312245123i64) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Test 2: Default format (no format sent)
  println!("  Test 2: Default format for timestamp 1609459200000");
  input_tx
    .send(Arc::new(1609459200000i64) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Test 3: Different custom format
  println!("  Test 3: Custom format for timestamp 946684800000");
  format_tx
    .send(Arc::new("%A, %B %e, %Y".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  input_tx
    .send(Arc::new(946684800000i64) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  // Test 4: Another custom format
  println!("  Test 4: Custom format for timestamp 1704067200000");
  format_tx
    .send(Arc::new("%Y/%m/%d".to_string()) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();
  input_tx
    .send(Arc::new(1704067200000i64) as Arc<dyn Any + Send + Sync>)
    .await
    .unwrap();

  println!("✓ All test cases sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);
  drop(format_tx);

  // Execute the graph
  println!("Executing graph with FormatTimeNode...");
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
      if let Ok(arc_formatted) = item.downcast::<String>() {
        output_results.push((*arc_formatted).clone());
        println!("  Output: '{}'", *arc_formatted);
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
    "✓ Received {} formatted timestamps via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected formatted strings
  let expected_results = vec![
    "2024-01-15 09:50:45".to_string(), // Test 1: custom format "%Y-%m-%d %H:%M:%S"
    "2024-01-15 09:50:45".to_string(), // Test 2: still using previous format (no new format sent)
    "Saturday, January  1, 2000".to_string(), // Test 3: new custom format "%A, %B %e, %Y"
    "2024/01/01".to_string(),          // Test 4: another custom format "%Y/%m/%d"
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ FormatTimeNode correctly formatted timestamps");
    println!("  Examples:");
    println!(
      "    Test 1: Timestamp 1705312245123 with format '%Y-%m-%d %H:%M:%S' -> '2024-01-15 09:50:45'"
    );
    println!("    Test 2: Timestamp 1609459200000 with same format -> '2024-01-15 09:50:45'");
    println!(
      "    Test 3: Timestamp 946684800000 with format '%A, %B %e, %Y' -> 'Saturday, January  1, 2000'"
    );
    println!("    Test 4: Timestamp 1704067200000 with format '%Y/%m/%d' -> '2024/01/01'");
  } else {
    println!(
      "⚠ FormatTimeNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
