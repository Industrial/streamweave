use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::while_loop_node::{WhileLoopConfig, WhileLoopNode, while_loop_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (data_tx, data_rx) = mpsc::channel(5);
  let (break_tx, break_rx) = mpsc::channel(5);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (break_out_tx, mut break_out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a configuration that can be sent to the graph's configuration port
  let loop_config: WhileLoopConfig = while_loop_config(
    |_value| async move {
      // Always continue looping - we'll exit via break signal
      Ok(true)
    },
    1000, // max iterations to prevent infinite loops
  );

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/while_loop_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("WhileLoopNode", |id, _inputs, _outputs| {
      Box::new(WhileLoopNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", data_rx)?;
  graph.connect_input_channel("condition", break_rx)?;
  graph.connect_output_channel("out", out_tx)?;
  graph.connect_output_channel("break", break_out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with graph! macro and connected channels");

  // Send configuration and input data to the channels AFTER building
  let _ = config_tx
    .send(Arc::new(Arc::new(loop_config)) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: an item that will loop indefinitely until break signal
  let _ = data_tx
    .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send a break signal to interrupt the loop
  let _ = break_tx
    .send(Arc::new("break".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

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
  drop(data_tx);
  drop(break_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut completed_count = 0;
  let mut broken_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let break_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), break_out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result
      && let Ok(num_arc) = item.downcast::<i32>()
    {
      let num = *num_arc;
      println!("  Completed loop: final value {}", num);
      completed_count += 1;
      has_data = true;
    }

    if let Ok(Some(item)) = break_result
      && let Ok(num_arc) = item.downcast::<i32>()
    {
      let num = *num_arc;
      println!("  Broken loop: interrupted at value {}", num);
      broken_count += 1;
      has_data = true;
    }

    if let Ok(Some(item)) = error_result
      && let Ok(error_msg) = item.downcast::<String>()
    {
      let error = &**error_msg;
      println!("  Error: {}", error);
      error_count += 1;
      has_data = true;
    }

    if !has_data {
      break;
    }
  }

  println!(
    "✓ Received {} items that completed normally via output channel",
    completed_count
  );
  println!(
    "✓ Received {} items that broke early via break channel",
    broken_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
