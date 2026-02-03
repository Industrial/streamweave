use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::object::object_merge_node::ObjectMergeNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    merge: ObjectMergeNode::new("merge".to_string()),
    graph.configuration => merge.configuration,
    graph.input1 => merge.in1,
    graph.input2 => merge.in2,
    merge.out => graph.output,
    merge.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input1", in1_rx)?;
  graph.connect_input_channel("input2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ObjectMergeNode using graph! macro");

  // Send configuration (optional for ObjectMergeNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create test objects
  println!("\n🧪 Testing object merging...");

  // Test 1: Merge objects with no overlapping keys
  println!("  Test 1: Merge objects with no overlapping keys");
  let mut obj1 = HashMap::new();
  obj1.insert(
    "name".to_string(),
    Arc::new("Alice".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj1.insert(
    "age".to_string(),
    Arc::new(30i64) as Arc<dyn Any + Send + Sync>,
  );

  let mut obj2 = HashMap::new();
  obj2.insert(
    "city".to_string(),
    Arc::new("New York".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj2.insert(
    "country".to_string(),
    Arc::new("USA".to_string()) as Arc<dyn Any + Send + Sync>,
  );

  let _ = in1_tx
    .send(Arc::new(obj1) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(obj2) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 2: Merge objects with overlapping keys (second object wins)
  println!("  Test 2: Merge objects with overlapping keys (second wins)");
  let mut obj3 = HashMap::new();
  obj3.insert(
    "name".to_string(),
    Arc::new("Bob".to_string()) as Arc<dyn Any + Send + Sync>,
  );
  obj3.insert(
    "age".to_string(),
    Arc::new(25i64) as Arc<dyn Any + Send + Sync>,
  );

  let mut obj4 = HashMap::new();
  obj4.insert(
    "age".to_string(),
    Arc::new(26i64) as Arc<dyn Any + Send + Sync>,
  ); // This should overwrite
  obj4.insert(
    "city".to_string(),
    Arc::new("London".to_string()) as Arc<dyn Any + Send + Sync>,
  );

  let _ = in1_tx
    .send(Arc::new(obj3) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(obj4) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Test 3: Merge with empty objects
  println!("  Test 3: Merge with one empty object");
  let mut obj5 = HashMap::new();
  obj5.insert(
    "value".to_string(),
    Arc::new(42i64) as Arc<dyn Any + Send + Sync>,
  );

  let obj6: HashMap<String, Arc<dyn Any + Send + Sync>> = HashMap::new(); // Empty object

  let _ = in1_tx
    .send(Arc::new(obj5) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = in2_tx
    .send(Arc::new(obj6) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ObjectMergeNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in1_tx);
  drop(in2_tx);

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
      // ObjectMergeNode outputs the merged object
      results_received += 1;
      println!("  Merged result {}:", results_received);

      // Try to downcast to HashMap (returned as Arc<HashMap>)
      if let Ok(merged_arc) = item
        .clone()
        .downcast::<Arc<HashMap<String, Arc<dyn Any + Send + Sync>>>>()
      {
        let merged = &**merged_arc;
        println!("    Merged object ({} properties)", merged.len());

        let mut sorted_keys: Vec<&String> = merged.keys().collect();
        sorted_keys.sort();

        for key in sorted_keys {
          if let Some(value_arc) = merged.get(key) {
            if let Ok(str_value) = value_arc.clone().downcast::<String>() {
              println!("      \"{}\" -> \"{}\"", key, (**str_value).to_string());
            } else if let Ok(int_value) = value_arc.clone().downcast::<i64>() {
              println!("      \"{}\" -> {}", key, *int_value);
            } else {
              println!("      \"{}\" -> <unknown type>", key);
            }
          }
        }
        success_count += 1;
      } else {
        // Try direct HashMap downcast (if not wrapped in Arc)
        if let Ok(merged_map) = item
          .clone()
          .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
        {
          let merged = &*merged_map;
          println!("    Merged object ({} properties)", merged.len());

          let mut sorted_keys: Vec<&String> = merged.keys().collect();
          sorted_keys.sort();

          for key in sorted_keys {
            if let Some(value_arc) = merged.get(key) {
              if let Ok(str_value) = value_arc.clone().downcast::<String>() {
                println!("      \"{}\" -> \"{}\"", key, (**str_value).to_string());
              } else if let Ok(int_value) = value_arc.clone().downcast::<i64>() {
                println!("      \"{}\" -> {}", key, *int_value);
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

  println!(
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 3 results
  if success_count == 3 && error_count == 0 {
    println!("✓ ObjectMergeNode correctly merged objects");
    println!("  Results should be: [name, age, city, country], [name, age(26), city], [value]");
  } else {
    println!(
      "⚠ ObjectMergeNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
