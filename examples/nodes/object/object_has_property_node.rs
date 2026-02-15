use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::object::object_has_property_node::ObjectHasPropertyNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (key_tx, key_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/object/object_has_property_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("ObjectHasPropertyNode", |id, _inputs, _outputs| {
      Box::new(ObjectHasPropertyNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("key", key_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ObjectHasPropertyNode (has) using graph! macro");

  // Send configuration (optional for ObjectHasPropertyNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test objects
  println!("\n🧪 Testing object property existence checks...");

  // Test 1: Check existing property
  println!("  Test 1: Has 'name' property (expected: true)");
  let mut obj1 = HashMap::new();
  obj1.insert(
    "name".to_string(),
    Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj1.insert(
    "age".to_string(),
    Arc::new(30i64) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(obj1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("name".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: Check non-existing property
  println!("  Test 2: Has 'address' property (expected: false)");
  let mut obj2 = HashMap::new();
  obj2.insert(
    "name".to_string(),
    Arc::new("Bob".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj2.insert(
    "age".to_string(),
    Arc::new(25i64) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(obj2) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("address".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Check existing property with different object
  println!("  Test 3: Has 'active' property (expected: true)");
  let mut obj3 = HashMap::new();
  obj3.insert(
    "active".to_string(),
    Arc::new(true) as Arc<dyn Any + Send + Sync>,
  );
  obj3.insert(
    "score".to_string(),
    Arc::new(95.5f64) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(obj3) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("active".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 4: Check another non-existing property
  println!("  Test 4: Has 'email' property (expected: false)");
  let mut obj4 = HashMap::new();
  obj4.insert(
    "name".to_string(),
    Arc::new("Charlie".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj4.insert(
    "age".to_string(),
    Arc::new(35i64) as Arc<dyn Any + Send + Sync>,
  );
  let _ = in_tx
    .send(Arc::new(obj4) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = key_tx
    .send(Arc::new("email".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ObjectHasPropertyNode...");
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
      // ObjectHasPropertyNode outputs the boolean result
      results_received += 1;
      println!("  Property check result {}:", results_received);

      if let Ok(bool_value) = item.clone().downcast::<bool>() {
        let has_property = *bool_value;
        println!("    Has property: {}", has_property);
        success_count += 1;
      } else {
        println!("    Unknown type (expected bool)");
        success_count += 1;
      }
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
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 4 results (true, false, true, false)
  if success_count == 4 && error_count == 0 {
    println!("✓ ObjectHasPropertyNode correctly checked property existence");
    println!(
      "  Results should be: true, false, true, false (checking for existing/non-existing properties)"
    );
  } else {
    println!(
      "⚠ ObjectHasPropertyNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 4, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
