use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::arithmetic::DivideNode;
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
  let mut graph = Graph::new("divide_example".to_string());
  graph.add_node(
    "divide".to_string(),
    Box::new(DivideNode::new("divide".to_string())),
  )?;
  graph.expose_input_port("divide", "configuration", "configuration")?;
  graph.expose_input_port("divide", "in1", "in1")?;
  graph.expose_input_port("divide", "in2", "in2")?;
  graph.expose_output_port("divide", "out", "output")?;
  graph.expose_output_port("divide", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with DivideNode using Graph API");

  // Send configuration (optional for DivideNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of numbers to divide (in1 / in2)
  let test_pairs = vec![(20i32, 4i32), (100i32, 5i32), (45i32, 9i32)]; // Results: 5, 20, 5
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
  println!("Executing graph with DivideNode...");
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
      // DivideNode outputs the quotient in various numeric types
      if let Ok(quotient_i32) = item.clone().downcast::<i32>() {
        let quotient = *quotient_i32;
        println!("  Quotient (i32): {}", quotient);
        success_count += 1;
        has_data = true;
      } else if let Ok(quotient_i64) = item.clone().downcast::<i64>() {
        let quotient = *quotient_i64;
        println!("  Quotient (i64): {}", quotient);
        success_count += 1;
        has_data = true;
      } else if let Ok(quotient_f32) = item.clone().downcast::<f32>() {
        let quotient = *quotient_f32;
        println!("  Quotient (f32): {:.2}", quotient);
        success_count += 1;
        has_data = true;
      } else if let Ok(quotient_f64) = item.downcast::<f64>() {
        let quotient = *quotient_f64;
        println!("  Quotient (f64): {:.2}", quotient);
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

  // Verify behavior: should receive 3 results (20/4=5, 100/5=20, 45/9=5)
  if success_count == 3 && error_count == 0 {
    println!("✓ DivideNode correctly performed division operations");
  } else {
    println!(
      "⚠ DivideNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
