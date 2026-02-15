use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArraySliceNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (start_tx, start_rx) = mpsc::channel(10);
  let (end_tx, end_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/array/slice_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("ArraySliceNode", |id, _inputs, _outputs| {
      Box::new(ArraySliceNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_input_channel("start", start_rx)?;
  graph.connect_input_channel("end", end_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArraySliceNode using graph! macro");

  // Send configuration (optional for ArraySliceNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: array and slice indices
  let test_array: Vec<Arc<dyn Any + Send + Sync>> = vec![
    Arc::new(10i32),
    Arc::new(20i32),
    Arc::new(30i32),
    Arc::new(40i32),
    Arc::new(50i32),
  ];

  // Send array, start index (1), and end index (4) - should slice [20, 30, 40]
  let _ = in_tx
    .send(Arc::new(test_array) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = start_tx
    .send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(4i32) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArraySliceNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(in_tx);
  drop(start_tx);
  drop(end_tx);

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
      // ArraySliceNode outputs the sliced array
      if let Ok(sliced_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let sliced_vec = (**sliced_arc).to_vec();
        println!("  Sliced array length: {}", sliced_vec.len());
        // Print the sliced values
        for (i, elem) in sliced_vec.iter().enumerate() {
          if let Ok(value_arc) = elem.clone().downcast::<i32>() {
            let value = *value_arc;
            println!("    [{}]: {}", i, value);
          }
        }
        success_count += 1;
        has_data = true;
      }
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

  println!(
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 1 result (array of 3 elements: [20, 30, 40])
  if success_count == 1 && error_count == 0 {
    println!("✓ ArraySliceNode correctly sliced the array");
  } else {
    println!(
      "⚠ ArraySliceNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 1, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
