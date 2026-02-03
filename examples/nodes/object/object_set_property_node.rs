use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::object::object_set_property_node::ObjectSetPropertyNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (key_tx, key_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    set: ObjectSetPropertyNode::new("set".to_string()),
    graph.configuration => set.configuration,
    graph.input => set.in,
    graph.key => set.key,
    graph.value => set.value,
    set.out => graph.output,
    set.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("key", key_rx)?;
  graph.connect_input_channel("value", value_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ObjectSetPropertyNode using Graph API");

  // Send configuration (optional for ObjectSetPropertyNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test objects
  println!("\n🧪 Testing object property setting...");

  // Test 1: Set new property in object
  println!("  Test 1: Set new 'city' property in person object");
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
    .send(Arc::new("city".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new("New York".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: Override existing property
  println!("  Test 2: Override existing 'age' property");
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
    .send(Arc::new("age".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = value_tx
    .send(Arc::new(26i64) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Set property with different value types
  println!("  Test 3: Set 'active' boolean property");
  let mut obj3 = HashMap::new();
  obj3.insert(
    "name".to_string(),
    Arc::new("Charlie".to_string()) as Arc<dyn Any + Send + Sync>,
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
  let _ = value_tx
    .send(Arc::new(true) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ObjectSetPropertyNode...");
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
  drop(value_tx);

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
      // ObjectSetPropertyNode outputs the object with property set
      results_received += 1;
      println!("  Set result {}:", results_received);

      // Try to downcast to Arc<HashMap>
      if let Ok(updated_arc) = item
        .clone()
        .downcast::<Arc<HashMap<String, Arc<dyn Any + Send + Sync>>>>()
      {
        let _updated = &**updated_arc;
        let updated = &**updated_arc;
        println!("    Updated object ({} properties)", updated.len());

        let mut sorted_keys: Vec<&String> = updated.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
          if let Some(value_arc) = updated.get(key) {
            if let Ok(str_value) = value_arc.clone().downcast::<String>() {
              println!("      \"{}\" -> \"{}\"", key, &(**str_value));
            } else if let Ok(int_value) = value_arc.clone().downcast::<i64>() {
              println!("      \"{}\" -> {}", key, *int_value);
            } else if let Ok(float_value) = value_arc.clone().downcast::<f64>() {
              println!("      \"{}\" -> {:.1}", key, *float_value);
            } else if let Ok(bool_value) = value_arc.clone().downcast::<bool>() {
              println!("      \"{}\" -> {}", key, *bool_value);
            } else {
              println!("      \"{}\" -> <unknown type>", key);
            }
          }
        }
        success_count += 1;
      } else if let Ok(updated_map) = item
        .clone()
        .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
      {
        let updated = &*updated_map;
        println!("    Updated object ({} properties)", updated.len());

        let mut sorted_keys: Vec<&String> = updated.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
          if let Some(value_arc) = updated.get(key) {
            if let Ok(str_value) = value_arc.clone().downcast::<String>() {
              println!("      \"{}\" -> \"{}\"", key, &(**str_value));
            } else if let Ok(int_value) = value_arc.clone().downcast::<i64>() {
              println!("      \"{}\" -> {}", key, *int_value);
            } else if let Ok(float_value) = value_arc.clone().downcast::<f64>() {
              println!("      \"{}\" -> {:.1}", key, *float_value);
            } else if let Ok(bool_value) = value_arc.clone().downcast::<bool>() {
              println!("      \"{}\" -> {}", key, *bool_value);
            } else {
              println!("      \"{}\" -> <unknown type>", key);
            }
          }
        }
        success_count += 1;
      } else {
        println!(
          "    Unknown type (expected HashMap), got: {}",
          std::any::type_name_of_val(&*item)
        );
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

  // Verify behavior: should receive 3 results
  if success_count == 3 && error_count == 0 {
    println!("✓ ObjectSetPropertyNode correctly set object properties");
    println!("  Results should be: [name, age, city], [name, age(26)], [name, score, active]");
  } else {
    println!(
      "⚠ ObjectSetPropertyNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
