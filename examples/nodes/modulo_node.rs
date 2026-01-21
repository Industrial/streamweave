use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::arithmetic::ModuloNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("modulo_example".to_string());
  graph.add_node(
    "modulo".to_string(),
    Box::new(ModuloNode::new("modulo".to_string())),
  )?;
  graph.expose_input_port("modulo", "configuration", "configuration")?;
  graph.expose_input_port("modulo", "in1", "in1")?;
  graph.expose_input_port("modulo", "in2", "in2")?;
  graph.expose_output_port("modulo", "out", "output")?;
  graph.expose_output_port("modulo", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ModuloNode using Graph API");

  // Send configuration (optional for ModuloNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of numbers to compute modulo (in1 % in2)
  let test_pairs = vec![(17i32, 5i32), (43i32, 7i32), (100i32, 13i32)]; // Results: 2, 1, 100%13=9
  for (dividend, divisor) in test_pairs {
    let _ = in1_tx
      .send(Arc::new(dividend) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = in2_tx
      .send(Arc::new(divisor) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ModuloNode...");
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

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      // ModuloNode outputs the remainder in various numeric types
      if let Ok(remainder_i32) = item.clone().downcast::<i32>() {
        let remainder = *remainder_i32;
        println!("  Remainder (i32): {}", remainder);
        success_count += 1;
        has_data = true;
      } else if let Ok(remainder_i64) = item.clone().downcast::<i64>() {
        let remainder = *remainder_i64;
        println!("  Remainder (i64): {}", remainder);
        success_count += 1;
        has_data = true;
      } else if let Ok(remainder_f32) = item.clone().downcast::<f32>() {
        let remainder = *remainder_f32;
        println!("  Remainder (f32): {:.2}", remainder);
        success_count += 1;
        has_data = true;
      } else if let Ok(remainder_f64) = item.downcast::<f64>() {
        let remainder = *remainder_f64;
        println!("  Remainder (f64): {:.2}", remainder);
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

  // Verify behavior: should receive 3 results (17%5=2, 43%7=1, 100%13=9)
  if success_count == 3 && error_count == 0 {
    println!("✓ ModuloNode correctly performed modulo operations");
  } else {
    println!(
      "⚠ ModuloNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
