use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::arithmetic::PowerNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (base_tx, base_rx) = mpsc::channel(10);
  let (exponent_tx, exponent_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("power_example".to_string());
  graph.add_node(
    "power".to_string(),
    Box::new(PowerNode::new("power".to_string())),
  )?;
  graph.expose_input_port("power", "configuration", "configuration")?;
  graph.expose_input_port("power", "base", "base")?;
  graph.expose_input_port("power", "exponent", "exponent")?;
  graph.expose_output_port("power", "out", "output")?;
  graph.expose_output_port("power", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("base", base_rx)?;
  graph.connect_input_channel("exponent", exponent_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with PowerNode using Graph API");

  // Send configuration (optional for PowerNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of base and exponent
  let test_pairs = vec![(2i32, 3i32), (5i32, 2i32), (3i32, 4i32)]; // Results: 8, 25, 81
  for (base, exponent) in test_pairs {
    let _ = base_tx
      .send(Arc::new(base) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = exponent_tx
      .send(Arc::new(exponent) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with PowerNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(base_tx);
  drop(exponent_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut success_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // PowerNode outputs the result in various numeric types
      if let Ok(result_i32) = item.clone().downcast::<i32>() {
        let result = *result_i32;
        println!("  Result (i32): {}", result);
        success_count += 1;
        has_data = true;
      } else if let Ok(result_i64) = item.clone().downcast::<i64>() {
        let result = *result_i64;
        println!("  Result (i64): {}", result);
        success_count += 1;
        has_data = true;
      } else if let Ok(result_f32) = item.clone().downcast::<f32>() {
        let result = *result_f32;
        println!("  Result (f32): {:.2}", result);
        success_count += 1;
        has_data = true;
      } else if let Ok(result_f64) = item.downcast::<f64>() {
        let result = *result_f64;
        println!("  Result (f64): {:.2}", result);
        success_count += 1;
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

  println!("✓ Received {} successful results via output channel", success_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 3 results (2^3=8, 5^2=25, 3^4=81)
  if success_count == 3 && error_count == 0 {
    println!("✓ PowerNode correctly performed exponentiation operations");
  } else {
    println!(
      "⚠ PowerNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
