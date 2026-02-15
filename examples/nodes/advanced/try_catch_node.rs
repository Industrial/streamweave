use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::advanced::try_catch_node::{
  CatchConfig, TryCatchNode, TryConfig, catch_config, try_config,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (try_tx, try_rx) = mpsc::channel(10);
  let (catch_tx, catch_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create try configuration that processes numbers but fails on negative values
  let try_config: TryConfig = try_config(|value| async move {
    if let Ok(num_arc) = value.downcast::<i32>() {
      let num = *num_arc;
      if num < 0 {
        Err(Arc::new(format!("Negative number not allowed: {}", num)) as Arc<dyn Any + Send + Sync>)
      } else {
        Ok(Arc::new(format!("Processed: {}", num * 2)) as Arc<dyn Any + Send + Sync>)
      }
    } else {
      Err(Arc::new("Expected i32".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });

  // Create catch configuration that handles errors by converting them to recovery messages
  let catch_config: CatchConfig = catch_config(|error| async move {
    if let Ok(error_msg_arc) = error.downcast::<String>() {
      let error_msg = (**error_msg_arc).to_string();
      Ok(Arc::new(format!("Recovered from error: {}", error_msg)) as Arc<dyn Any + Send + Sync>)
    } else {
      Err(Arc::new("Could not handle error".to_string()) as Arc<dyn Any + Send + Sync>)
    }
  });

  // Build the graph from Mermaid (.mmd)
  let mut graph: Graph = {
    use std::path::Path;
    use streamweave::mermaid::{
      NodeRegistry, blueprint_to_graph::blueprint_to_graph, parse::parse_mmd_file_to_blueprint,
    };
    let path = Path::new("examples/nodes/advanced/try_catch_node.mmd");
    let bp = parse_mmd_file_to_blueprint(path).map_err(|e| e.to_string())?;
    let mut registry = NodeRegistry::new();
    registry.register("TryCatchNode", |id, _inputs, _outputs| {
      Box::new(TryCatchNode::new(id))
    });
    blueprint_to_graph(&bp, Some(&registry)).map_err(|e| e.to_string())?
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_input_channel("try", try_rx)?;
  graph.connect_input_channel("catch", catch_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with TryCatchNode using graph! macro");

  // Send configuration (optional for TryCatchNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send try function
  let _ = try_tx
    .send(Arc::new(try_config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send catch function
  let _ = catch_tx
    .send(Arc::new(catch_config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  // Send test data: mix of positive and negative numbers
  let test_data = vec![5, -3, 10, -7, 2]; // 5, 10, 2 will succeed; -3, -7 will fail but be caught
  for num in test_data {
    let _ = input_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with TryCatchNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(try_tx);
  drop(catch_tx);

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

    if let Ok(Some(item)) = out_result
      && let Ok(result_arc) = item.downcast::<String>()
    {
      let result = (**result_arc).to_string();
      println!("  Success: {}", result);
      success_count += 1;
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

  println!(
    "✓ Received {} successful results via output channel",
    success_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 5 results (3 successful processing + 2 recovered errors)
  if success_count == 5 && error_count == 0 {
    println!("✓ TryCatchNode correctly processed items with error recovery");
  } else {
    println!(
      "⚠ TryCatchNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 5, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
