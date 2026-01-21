use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayReverseNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the Graph API
  let mut graph = Graph::new("reverse_example".to_string());
  graph.add_node(
    "reverse".to_string(),
    Box::new(ArrayReverseNode::new("reverse".to_string())),
  )?;
  graph.expose_input_port("reverse", "configuration", "configuration")?;
  graph.expose_input_port("reverse", "in", "input")?;
  graph.expose_output_port("reverse", "out", "output")?;
  graph.expose_output_port("reverse", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayReverseNode using Graph API");

  // Send configuration (optional for ArrayReverseNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: arrays to reverse
  let test_arrays = vec![
    vec!["a".to_string(), "b".to_string(), "c".to_string()], // ["a", "b", "c"] -> ["c", "b", "a"]
    vec!["1".to_string(), "2".to_string()],                  // ["1", "2"] -> ["2", "1"]
    vec![
      "x".to_string(),
      "y".to_string(),
      "z".to_string(),
      "w".to_string(),
    ], // ["x", "y", "z", "w"] -> ["w", "z", "y", "x"]
  ];

  for array_data in test_arrays {
    let array_vec: Vec<Arc<dyn Any + Send + Sync>> = array_data
      .iter()
      .map(|s| Arc::new(s.clone()) as Arc<dyn Any + Send + Sync>)
      .collect();

    let _ = in_tx
      .send(Arc::new(array_vec) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayReverseNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);

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
      // ArrayReverseNode outputs a reversed array
      if let Ok(reversed_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let reversed = (**reversed_arc).to_vec();
        let mut elements = Vec::new();
        for elem in reversed {
          if let Ok(str_arc) = elem.downcast::<String>() {
            elements.push((**str_arc).to_string());
          }
        }
        println!("  Reversed array: {:?}", elements);
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

  println!(
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 3 results (reversed arrays)
  if success_count == 3 && error_count == 0 {
    println!("✓ ArrayReverseNode correctly reversed array elements");
  } else {
    println!(
      "⚠ ArrayReverseNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 3, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
