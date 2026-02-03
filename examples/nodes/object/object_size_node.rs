use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::object::object_size_node::ObjectSizeNode;
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
    object_size: ObjectSizeNode::new("object_size".to_string()),
    graph.configuration => object_size.configuration,
    graph.input => object_size.in,
    object_size.out => graph.output,
    object_size.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ObjectSizeNode using graph! macro");

  // Send configuration (optional for ObjectSizeNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test objects with different sizes
  println!("📥 Sending objects to measure their sizes");
  let test_objects = vec![
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
      obj.insert(
        "active".to_string(),
        Arc::new(true) as Arc<dyn Any + Send + Sync>,
      );
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    },
    {
      let mut obj = HashMap::new();
      obj.insert(
        "single".to_string(),
        Arc::new("value".to_string()) as Arc<dyn Any + Send + Sync>,
      );
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    },
    {
      let obj: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new(); // Empty object
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    },
    {
      let mut obj = HashMap::new();
      obj.insert(
        "x".to_string(),
        Arc::new(1i64) as Arc<dyn Any + Send + Sync>,
      );
      obj.insert(
        "y".to_string(),
        Arc::new(2i64) as Arc<dyn Any + Send + Sync>,
      );
      obj.insert(
        "z".to_string(),
        Arc::new(3i64) as Arc<dyn Any + Send + Sync>,
      );
      obj.insert(
        "w".to_string(),
        Arc::new(4i64) as Arc<dyn Any + Send + Sync>,
      );
      Arc::new(obj) as Arc<dyn Any + Send + Sync>
    },
  ];

  let expected_sizes = vec![3, 1, 0, 4];
  let descriptions = [
    "Object with 3 properties (name, age, active)",
    "Object with 1 property (single)",
    "Empty object (0 properties)",
    "Object with 4 properties (x, y, z, w)",
  ];

  for (i, obj) in test_objects.into_iter().enumerate() {
    println!(
      "  Sending: {} -> expected size {}",
      descriptions[i], expected_sizes[i]
    );
    input_tx.send(obj).await.unwrap();
  }

  println!("✓ All test objects sent");

  // Close input channels to signal end of data
  drop(config_tx);
  drop(input_tx);

  // Execute the graph
  println!("Executing graph with ObjectSizeNode...");
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
      && let Ok(arc_size) = item.downcast::<i64>()
    {
      output_results.push(*arc_size);
      println!("  Output: object size = {}", *arc_size);
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
    "✓ Received {} size measurements via output channel",
    output_results.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive expected sizes
  if output_results == expected_sizes && error_count == 0 {
    println!("✓ ObjectSizeNode correctly measured object sizes");
    println!("  Results:");
    for (i, &size) in output_results.iter().enumerate() {
      println!("    {} -> size {}", descriptions[i], size);
    }
  } else {
    println!(
      "⚠ ObjectSizeNode behavior may be unexpected (received: {:?}, expected: {:?}, errors: {})",
      output_results, expected_sizes, error_count
    );
  }

  Ok(())
}
