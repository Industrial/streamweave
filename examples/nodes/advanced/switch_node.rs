use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::advanced::switch_node::{SwitchConfig, SwitchNode, switch_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (value_tx, value_rx) = mpsc::channel(10);
  let (out0_tx, mut out0_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (out1_tx, mut out1_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (default_tx, mut default_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a switch configuration that routes based on switch value ranges
  let switch_config: SwitchConfig = switch_config(|_data, value| async move {
    // Extract the switch value as an integer
    if let Ok(value_arc) = value.downcast::<i32>() {
      let switch_value = *value_arc;

      // Route based on ranges:
      // 0-9 -> out_0, 10-19 -> out_1, others -> default
      if (0..10).contains(&switch_value) {
        Ok(Some(0))
      } else if (10..20).contains(&switch_value) {
        Ok(Some(1))
      } else {
        Ok(None) // route to default
      }
    } else {
      Err("Expected i32 for switch value".to_string())
    }
  });

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/advanced/switch_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("SwitchNode", |id, _inputs, _outputs| {
      Box::new(SwitchNode::new(id, 2))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("value", value_rx)?;
  graph.connect_output_channel("out_0", out0_tx)?;
  graph.connect_output_channel("out_1", out1_tx)?;
  graph.connect_output_channel("default", default_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with SwitchNode using graph! macro");

  // Send configuration
  let _ = config_tx
    .send(Arc::new(switch_config) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data with corresponding switch values
  let test_cases = vec![
    ("data_A", 5),  // 5 -> out_0 (0-9 range)
    ("data_B", 15), // 15 -> out_1 (10-19 range)
    ("data_C", 25), // 25 -> default (other values)
    ("data_D", 8),  // 8 -> out_0 (0-9 range)
    ("data_E", 12), // 12 -> out_1 (10-19 range)
  ];

  for (data, switch_value) in test_cases {
    let _ = input_tx
      .send(Arc::new(data.to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = value_tx
      .send(Arc::new(switch_value) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with SwitchNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(value_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut out0_count = 0;
  let mut out1_count = 0;
  let mut default_count = 0;
  let mut error_count = 0;

  loop {
    let out0_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out0_rx.recv()).await;
    let out1_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out1_rx.recv()).await;
    let default_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), default_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out0_result
      && let Ok(data_arc) = item.downcast::<String>()
    {
      let data = (**data_arc).to_string();
      println!("  Out_0: {}", data);
      out0_count += 1;
      has_data = true;
    }

    if let Ok(Some(item)) = out1_result
      && let Ok(data_arc) = item.downcast::<String>()
    {
      let data = (**data_arc).to_string();
      println!("  Out_1: {}", data);
      out1_count += 1;
      has_data = true;
    }

    if let Ok(Some(item)) = default_result
      && let Ok(data_arc) = item.downcast::<String>()
    {
      let data = (**data_arc).to_string();
      println!("  Default: {}", data);
      default_count += 1;
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

  println!("✓ Received {} items via out_0 channel", out0_count);
  println!("✓ Received {} items via out_1 channel", out1_count);
  println!("✓ Received {} items via default channel", default_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 2 items on out_0, 2 on out_1, 1 on default
  if out0_count == 2 && out1_count == 2 && default_count == 1 && error_count == 0 {
    println!("✓ SwitchNode correctly routed items based on switch values");
  } else {
    println!(
      "⚠ SwitchNode behavior may be unexpected (out0: {}, out1: {}, default: {}, errors: {}, expected out0: 2, out1: 2, default: 1, errors: 0)",
      out0_count, out1_count, default_count, error_count
    );
  }

  Ok(())
}
