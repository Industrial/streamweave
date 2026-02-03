use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::condition_node::{ConditionConfig, ConditionNode, condition_config};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (true_tx, mut true_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (false_tx, mut false_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Create a configuration that routes positive numbers (including zero) to "true" and negative to "false"
  let condition_config: ConditionConfig = condition_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      let num = *arc_i32;
      Ok(num >= 0)
    } else {
      Err("Expected i32".to_string())
    }
  });

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    condition: ConditionNode::new("condition".to_string()),
    graph.configuration => condition.configuration,
    graph.input => condition.in,
    condition.true => graph.true,
    condition.false => graph.false,
    condition.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input", input_rx)?;
  graph.connect_output_channel("true", true_tx)?;
  graph.connect_output_channel("false", false_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ConditionNode using graph! macro");

  // Send configuration
  let _ = config_tx
    .send(Arc::new(condition_config) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test numbers to demonstrate conditional routing
  let test_numbers = vec![5, -3, 0, 10, -7, 2, -1];
  for num in test_numbers {
    let _ = input_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  println!("✓ Configuration sent and test data sent to input channel");

  // Execute the graph
  println!("Executing graph with ConditionNode...");
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
  let mut true_count = 0;
  let mut false_count = 0;
  let mut error_count = 0;

  loop {
    let true_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(50), true_rx.recv()).await;
    let false_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(50), false_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(50), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = true_result {
      if let Ok(num_arc) = item.downcast::<i32>() {
        let num = *num_arc;
        println!("  True: {}", num);
        true_count += 1;
        has_data = true;
      }
    }

    if let Ok(Some(item)) = false_result {
      if let Ok(num_arc) = item.downcast::<i32>() {
        let num = *num_arc;
        println!("  False: {}", num);
        false_count += 1;
        has_data = true;
      }
    }

    if let Ok(Some(item)) = error_result {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = &**error_msg;
        println!("  Error: {}", error);
        error_count += 1;
        has_data = true;
      }
    }

    if !has_data {
      break;
    }
  }

  println!("✓ Received {} numbers routed to 'true' output", true_count);
  println!(
    "✓ Received {} numbers routed to 'false' output",
    false_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 4 positive/zero numbers (5, 0, 10, 2) and 3 negative (-3, -7, -1)
  if true_count == 4 && false_count == 3 {
    println!("✓ ConditionNode correctly routed numbers based on condition (>= 0)");
  } else {
    println!(
      "⚠ ConditionNode behavior may be unexpected (true: {}, false: {}, expected true: 4, false: 3)",
      true_count, false_count
    );
  }

  Ok(())
}
