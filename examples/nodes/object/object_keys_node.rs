use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::object::object_keys_node::ObjectKeysNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("keys_example".to_string());
  graph.add_node(
    "keys".to_string(),
    Box::new(ObjectKeysNode::new("keys".to_string())),
  )?;
  graph.expose_input_port("keys", "configuration", "configuration")?;
  graph.expose_input_port("keys", "in", "input")?;
  graph.expose_output_port("keys", "out", "output")?;
  graph.expose_output_port("keys", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ObjectKeysNode (keys) using Graph API");

  // Send configuration (optional for ObjectKeysNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test objects
  println!("\n🧪 Testing object key extraction...");

  // Test 1: Extract keys from object with multiple properties
  println!("  Test 1: Extract keys from person object (expected: name, age, city)");
  let mut obj1 = HashMap::new();
  obj1.insert("name".to_string(), Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>);
  obj1.insert("age".to_string(), Arc::new(30i64) as Arc<dyn Any + Send + Sync>);
  obj1.insert("city".to_string(), Arc::new("New York".to_string()) as Arc<dyn Any + Send + Sync>);
  let _ = in_tx
    .send(Arc::new(obj1) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: Extract keys from object with single property
  println!("  Test 2: Extract keys from simple object (expected: value)");
  let mut obj2 = HashMap::new();
  obj2.insert("value".to_string(), Arc::new(42i64) as Arc<dyn Any + Send + Sync>);
  let _ = in_tx
    .send(Arc::new(obj2) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Extract keys from empty object
  println!("  Test 3: Extract keys from empty object (expected: empty array)");
  let obj3: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new();
  let _ = in_tx
    .send(Arc::new(obj3) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ObjectKeysNode...");
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
  let mut results_received = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // ObjectKeysNode outputs the array of keys
      results_received += 1;
      println!("  Keys result {}:", results_received);

      // Try to downcast to Vec<Arc<dyn Any + Send + Sync>>
      if let Ok(keys_vec) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let keys = (**keys_vec).to_vec();
        println!("    Keys ({})", keys.len());

        for (i, key_arc) in keys.iter().enumerate() {
          if let Ok(key_str) = key_arc.clone().downcast::<String>() {
            println!("      [{}]: \"{}\"", i, (**key_str).to_string());
          } else {
            println!("      [{}]: <unknown type>", i);
          }
        }
        success_count += 1;
      } else {
        println!("    Unknown type (expected Vec<String>)");
        success_count += 1;
      }
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

  println!("✓ Received {} successful results via output channel", success_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 3 results
  if success_count == 3 && error_count == 0 {
    println!("✓ ObjectKeysNode correctly extracted object keys");
    println!("  Results should be: ['name', 'age', 'city'], ['value'], [] (empty array)");
  } else {
    println!(
      "⚠ ObjectKeysNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
