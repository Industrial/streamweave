use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::error_branch_node::{ErrorBranchConfig, ErrorBranchNode};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (success_tx, mut success_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/advanced/error_branch_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("ErrorBranchNode", |id, _inputs, _outputs| {
      Box::new(ErrorBranchNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("success", success_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ErrorBranchNode using graph! macro");

  // Configuration is optional for ErrorBranchNode
  let config = Arc::new(ErrorBranchConfig {});

  // Send the configuration
  let _ = config_tx.send(config as Arc<dyn Any + Send + Sync>).await;

  // Send test results to demonstrate routing based on Result types
  let test_results: Vec<Result<Arc<dyn Any + Send + Sync>, String>> = vec![
    Ok(Arc::new("success_1".to_string()) as Arc<dyn Any + Send + Sync>),
    Err("error_1".to_string()),
    Ok(Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
    Err("error_2".to_string()),
    Ok(Arc::new("success_2".to_string()) as Arc<dyn Any + Send + Sync>),
    Err("error_3".to_string()),
  ];

  for result in test_results {
    let _ = input_tx
      .send(Arc::new(result) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  // Also send a non-Result type to demonstrate error handling
  let _ = input_tx
    .send(Arc::new("not_a_result".to_string()) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration sent and test data sent to input channel");

  // Execute the graph
  println!("Executing graph with ErrorBranchNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut success_count = 0;
  let mut error_count = 0;

  loop {
    let success_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), success_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = success_result {
      if let Ok(string_arc) = item.clone().downcast::<String>() {
        let value = (**string_arc).to_string();
        println!("  Success: {}", value);
        success_count += 1;
        has_data = true;
      } else if let Ok(int_arc) = item.downcast::<i32>() {
        let value = *int_arc;
        println!("  Success: {}", value);
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
    "✓ Received {} successful results via success channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 3 success values and 4 errors (3 explicit + 1 non-Result)
  if success_count == 3 && error_count == 4 {
    println!("✓ ErrorBranchNode correctly routed Result values by success/failure");
  } else {
    println!(
      "⚠ ErrorBranchNode behavior may be unexpected (success: {}, errors: {}, expected success: 3, errors: 4)",
      success_count, error_count
    );
  }

  Ok(())
}
