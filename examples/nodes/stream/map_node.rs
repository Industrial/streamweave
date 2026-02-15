use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::map_node::{MapConfig, MapNode, map_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(5);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a configuration that can be sent to the graph's configuration port
  let square_config: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let num = *arc_i32;
      Ok(Arc::new(num * num) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/stream/map_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("MapNode", |id, _inputs, _outputs| {
      Box::new(MapNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("output", output_tx)?;

  println!("✓ Graph built with graph! macro");

  // Send configuration and input data to the channels AFTER building
  let _ = config_tx
    .send(Arc::new(square_config) as Arc<dyn Any + Send + Sync>)
    .await;

  for num in 1..=3 {
    let _ = input_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
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

  // Read results from the output channel
  println!("Reading results from output channel...");
  let mut count = 0;
  loop {
    match tokio::time::timeout(tokio::time::Duration::from_millis(100), output_rx.recv()).await {
      Ok(Some(item)) => {
        if let Ok(arc_i32) = item.downcast::<i32>() {
          let result = *arc_i32;
          println!("  {}² = {}", count + 1, result);
          count += 1;
          if count >= 3 {
            break;
          }
        }
      }
      Ok(None) => {
        println!("Output channel closed");
        break;
      }
      Err(_) => {
        // Timeout, continue
      }
    }
  }

  println!("✓ Received {} results via connected channels", count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
