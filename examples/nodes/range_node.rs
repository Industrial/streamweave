use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::range_node::{RangeConfig, RangeNode};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (start_tx, start_rx) = mpsc::channel(10);
  let (end_tx, end_rx) = mpsc::channel(10);
  let (step_tx, step_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("range_example".to_string());
  graph.add_node(
    "range".to_string(),
    Box::new(RangeNode::new("range".to_string())),
  )?;
  graph.expose_input_port("range", "configuration", "configuration")?;
  graph.expose_input_port("range", "start", "start")?;
  graph.expose_input_port("range", "end", "end")?;
  graph.expose_input_port("range", "step", "step")?;
  graph.expose_output_port("range", "out", "output")?;
  graph.expose_output_port("range", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("start", start_rx)?;
  graph.connect_input_channel("end", end_rx)?;
  graph.connect_input_channel("step", step_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with RangeNode using Graph API");

  // Configuration is optional for RangeNode
  let config = Arc::new(RangeConfig {});

  // Send the configuration
  let _ = config_tx.send(config as Arc<dyn Any + Send + Sync>).await;

  // Send range parameters: generate numbers from 0 to 10 with step 2
  let _ = start_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let _ = end_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let _ = step_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration sent and range parameters sent to input channels");

  // Execute the graph
  println!("Executing graph with RangeNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(start_tx);
  drop(end_tx);
  drop(step_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut number_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      if let Ok(num_arc) = item.clone().downcast::<i32>() {
        let num = *num_arc;
        println!("  Generated: {}", num);
        number_count += 1;
        has_data = true;
      } else if let Ok(num_arc) = item.downcast::<f64>() {
        let num = *num_arc;
        println!("  Generated: {}", num);
        number_count += 1;
        has_data = true;
      }
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

  println!("✓ Received {} numbers via output channel", number_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive numbers 0, 2, 4, 6, 8 (5 numbers total)
  if number_count == 5 && error_count == 0 {
    println!("✓ RangeNode correctly generated sequence from 0 to 10 with step 2");
  } else {
    println!(
      "⚠ RangeNode behavior may be unexpected (numbers: {}, errors: {}, expected numbers: 5, errors: 0)",
      number_count, error_count
    );
  }

  Ok(())
}
