use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::arithmetic::MultiplyNode;
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
  let mut graph = Graph::new("multiply_example".to_string());
  graph.add_node(
    "multiply".to_string(),
    Box::new(MultiplyNode::new("multiply".to_string())),
  )?;
  graph.expose_input_port("multiply", "configuration", "configuration")?;
  graph.expose_input_port("multiply", "in1", "in1")?;
  graph.expose_input_port("multiply", "in2", "in2")?;
  graph.expose_output_port("multiply", "out", "output")?;
  graph.expose_output_port("multiply", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with MultiplyNode using Graph API");

  // Send configuration (optional for MultiplyNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of numbers to multiply
  let test_pairs = vec![(6i32, 7i32), (12i32, 8i32), (15i32, 4i32)]; // Results: 42, 96, 60
  for (a, b) in test_pairs {
    let _ = in1_tx
      .send(Arc::new(a) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = in2_tx
      .send(Arc::new(b) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with MultiplyNode...");
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
      // MultiplyNode outputs the product in various numeric types
      if let Ok(product_i32) = item.clone().downcast::<i32>() {
        let product = *product_i32;
        println!("  Product (i32): {}", product);
        success_count += 1;
        has_data = true;
      } else if let Ok(product_i64) = item.clone().downcast::<i64>() {
        let product = *product_i64;
        println!("  Product (i64): {}", product);
        success_count += 1;
        has_data = true;
      } else if let Ok(product_f32) = item.clone().downcast::<f32>() {
        let product = *product_f32;
        println!("  Product (f32): {:.2}", product);
        success_count += 1;
        has_data = true;
      } else if let Ok(product_f64) = item.downcast::<f64>() {
        let product = *product_f64;
        println!("  Product (f64): {:.2}", product);
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

  // Verify behavior: should receive 3 results (6*7=42, 12*8=96, 15*4=60)
  if success_count == 3 && error_count == 0 {
    println!("✓ MultiplyNode correctly performed multiplication operations");
  } else {
    println!(
      "⚠ MultiplyNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
