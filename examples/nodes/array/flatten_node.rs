use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayFlattenNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in_tx, in_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/array/flatten_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("ArrayFlattenNode", |id, _inputs, _outputs| {
      Box::new(ArrayFlattenNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", in_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayFlattenNode using graph! macro");

  // Send configuration (optional for ArrayFlattenNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: nested arrays to flatten
  let test_arrays = vec![
    // [[1, 2], [3, 4], 5] -> [1, 2, 3, 4, 5]
    vec![
      Arc::new(vec![
        Arc::new("1".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("2".to_string()) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
      Arc::new(vec![
        Arc::new("3".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("4".to_string()) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
      Arc::new("5".to_string()) as Arc<dyn Any + Send + Sync>,
    ],
    // [[a, b, c], [x, y]] -> [a, b, c, x, y]
    vec![
      Arc::new(vec![
        Arc::new("a".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("b".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("c".to_string()) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
      Arc::new(vec![
        Arc::new("x".to_string()) as Arc<dyn Any + Send + Sync>,
        Arc::new("y".to_string()) as Arc<dyn Any + Send + Sync>,
      ]) as Arc<dyn Any + Send + Sync>,
    ],
  ];

  for nested_array in test_arrays {
    let _ = in_tx
      .send(Arc::new(nested_array) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayFlattenNode...");
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
      // ArrayFlattenNode outputs a flattened array
      if let Ok(flattened_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let flattened = (**flattened_arc).to_vec();
        let mut elements = Vec::new();
        for elem in flattened {
          if let Ok(str_arc) = elem.downcast::<String>() {
            elements.push((**str_arc).to_string());
          }
        }
        println!("  Flattened array: {:?}", elements);
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

  // Verify behavior: should receive 2 results (flattened arrays)
  if success_count == 2 && error_count == 0 {
    println!("✓ ArrayFlattenNode correctly flattened nested arrays");
  } else {
    println!(
      "⚠ ArrayFlattenNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 2, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
