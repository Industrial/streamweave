use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::for_each_node::{ForEachConfig, ForEachNode, for_each_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(20);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a configuration that extracts Vec<i32> and emits each item
  let for_each_config: ForEachConfig = for_each_config(|value| async move {
    if let Ok(arc_vec) = value.downcast::<Vec<i32>>() {
      let items: Vec<Arc<dyn Any + Send + Sync>> = arc_vec
        .iter()
        .map(|&n| Arc::new(n) as Arc<dyn Any + Send + Sync>)
        .collect();
      Ok(items)
    } else {
      Err("Expected Vec<i32>".to_string())
    }
  });

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/stream/for_each_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("ForEachNode", |id, _inputs, _outputs| {
      Box::new(ForEachNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with connected channels");

  // Send configuration and input data to the channels AFTER building
  let _ = config_tx
    .send(Arc::new(for_each_config) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send collections to be expanded
  let collections = vec![
    vec![1, 2, 3],    // 3 items
    vec![4, 5],       // 2 items
    vec![6, 7, 8, 9], // 4 items
  ];

  for collection in collections {
    let _ = input_tx
      .send(Arc::new(collection) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  println!("✓ Data sent to channels");

  // Execute the graph (channels have data now)
  println!("Executing graph...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);

  // Read expanded items from the output channel
  println!("Reading expanded items from output channel...");
  let mut item_count = 0;
  let mut timeout_count = 0;
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), output_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(arc_i32) = item.downcast::<i32>() {
          let result = *arc_i32;
          println!("  Item: {}", result);
          item_count += 1;
          timeout_count = 0; // Reset timeout counter on successful receive
        }
      }
      Ok(None) => {
        println!("Output channel closed");
        break;
      }
      Err(_) => {
        // Timeout, break after a few consecutive timeouts
        timeout_count += 1;
        if timeout_count >= 5 {
          println!("No more items (timed out after {} attempts)", timeout_count);
          break;
        }
      }
    }
  }

  // Read errors from the error channel (should be empty for this example)
  println!("Reading errors from error channel...");
  let mut error_count = 0;
  let mut timeout_count = 0;
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(error_msg) = item.downcast::<String>() {
          let error = &**error_msg;
          println!("  Error: {}", error);
          error_count += 1;
        }
      }
      Ok(None) => {
        println!("Error channel closed");
        break;
      }
      Err(_) => {
        // Timeout, break after a few timeouts since we don't expect errors
        timeout_count += 1;
        if timeout_count >= 5 {
          println!(
            "No more errors (timed out after {} attempts)",
            timeout_count
          );
          break;
        }
      }
    }
  }

  println!(
    "✓ Received {} expanded items via output channel",
    item_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
