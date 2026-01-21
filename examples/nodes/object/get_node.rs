use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::object::ObjectPropertyNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (key_tx, key_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("get_example".to_string());
  graph.add_node(
    "get".to_string(),
    Box::new(ObjectPropertyNode::new("get".to_string())),
  )?;
  graph.expose_input_port("get", "configuration", "configuration")?;
  graph.expose_input_port("get", "in", "input")?;
  graph.expose_input_port("get", "key", "key")?;
  graph.expose_output_port("get", "out", "output")?;
  graph.expose_output_port("get", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("key", key_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ObjectPropertyNode (get) using Graph API");

  // Send configuration (optional for ObjectPropertyNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test objects
  println!("\n🧪 Testing object property retrieval...");

  // Test 1: Get existing string property
  println!("  Test 1: Get 'name' from object (expected: 'Alice')");
  let mut obj1 = HashMap::new();
  obj1.insert("name".to_string(), Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>);
  obj1.insert("age".to_string(), Arc::new(30i64) as Arc<dyn Any + Send + Sync>);
  let _ = in_tx
    .send(Arc::new(obj1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("name".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: Get existing integer property
  println!("  Test 2: Get 'age' from object (expected: 30)");
  let mut obj2 = HashMap::new();
  obj2.insert("name".to_string(), Arc::new("Bob".to_string()) as Arc<dyn Any + Send + Sync>);
  obj2.insert("age".to_string(), Arc::new(25i64) as Arc<dyn Any + Send + Sync>);
  let _ = in_tx
    .send(Arc::new(obj2) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("age".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Get non-existing property
  println!("  Test 3: Get 'address' from object (expected: error)");
  let mut obj3 = HashMap::new();
  obj3.insert("name".to_string(), Arc::new("Charlie".to_string()) as Arc<dyn Any + Send + Sync>);
  obj3.insert("age".to_string(), Arc::new(35i64) as Arc<dyn Any + Send + Sync>);
  let _ = in_tx
    .send(Arc::new(obj3) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("address".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: Get boolean property
  println!("  Test 4: Get 'active' from object (expected: true)");
  let mut obj4 = HashMap::new();
  obj4.insert("active".to_string(), Arc::new(true) as Arc<dyn Any + Send + Sync>);
  obj4.insert("score".to_string(), Arc::new(95.5f64) as Arc<dyn Any + Send + Sync>);
  let _ = in_tx
    .send(Arc::new(obj4) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("active".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ObjectPropertyNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(key_tx);

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
      // ObjectPropertyNode outputs the property value
      results_received += 1;
      println!("  Property result {}:", results_received);

      // Try different value types
      if let Ok(str_value) = item.clone().downcast::<String>() {
        let value = (**str_value).to_string();
        println!("    String: {}", value);
        success_count += 1;
      } else if let Ok(int_value) = item.clone().downcast::<i64>() {
        let value = *int_value;
        println!("    i64: {}", value);
        success_count += 1;
      } else if let Ok(bool_value) = item.clone().downcast::<bool>() {
        let value = *bool_value;
        println!("    bool: {}", value);
        success_count += 1;
      } else if let Ok(float_value) = item.clone().downcast::<f64>() {
        let value = *float_value;
        println!("    f64: {:.1}", value);
        success_count += 1;
      } else {
        println!("    Unknown type");
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

  // Verify behavior: should receive 3 results and 1 error
  if success_count == 3 && error_count == 1 {
    println!("✓ ObjectPropertyNode correctly retrieved object properties");
    println!("  Results should be: 'Alice', 25, true (with 1 error for missing 'address' property)");
  } else {
    println!(
      "⚠ ObjectPropertyNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 1)",
      success_count, error_count
    );
  }

  Ok(())
}
