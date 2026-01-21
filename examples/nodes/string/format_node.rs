use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::string::StringFormatNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (template_tx, template_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("format_example".to_string());
  graph.add_node(
    "format".to_string(),
    Box::new(StringFormatNode::new("format".to_string())),
  )?;
  graph.expose_input_port("format", "configuration", "configuration")?;
  graph.expose_input_port("format", "template", "template")?;
  graph.expose_input_port("format", "value", "value")?;
  graph.expose_output_port("format", "out", "output")?;
  graph.expose_output_port("format", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("template", template_rx)?;
  graph.connect_input_channel("value", value_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with StringFormatNode using Graph API");

  // Send configuration (optional for StringFormatNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: templates and values to format
  println!("📥 Sending templates and values to format");
  let test_cases = vec![
    ("Hello {}", "World".to_string()),
    ("Count: {}", 42.to_string()),
    ("User: {} logged in", "Alice".to_string()),
    ("Price: ${}", "29.99".to_string()),
    ("Status: {}", "Active".to_string()),
  ];

  for (template, value_str) in test_cases {
    println!("  Formatting: '{}' with '{}'", template, value_str);

    template_tx
      .send(Arc::new(template.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();

    value_tx
      .send(Arc::new(value_str.clone()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to input channels");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(template_tx);
  drop(value_tx);

  // Execute the graph
  println!("Executing graph with StringFormatNode...");
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
      if let Ok(result_str) = item.downcast::<String>() {
        output_results.push((*result_str).clone());
        println!("  Output: '{}'", *result_str);
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
    "✓ Received {} results via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected formatted results
  let expected_results = vec![
    "Hello World".to_string(),
    "Count: 42".to_string(),
    "User: Alice logged in".to_string(),
    "Price: $29.99".to_string(),
    "Status: Active".to_string(),
  ];

  if output_results == expected_results && error_count == 0 {
    println!("✓ StringFormatNode correctly formatted strings with placeholders");
  } else {
    println!(
      "⚠ StringFormatNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_results, expected_results, error_count
    );
  }

  Ok(())
}
