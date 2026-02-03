use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::array::ArrayConcatNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (in1_tx, in1_rx) = mpsc::channel(10);
  let (in2_tx, in2_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    concat: ArrayConcatNode::new("concat".to_string()),
    graph.configuration => concat.configuration,
    graph.in1 => concat.in1,
    graph.in2 => concat.in2,
    concat.out => graph.output,
    concat.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("in1", in1_rx)?;
  graph.connect_input_channel("in2", in2_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ArrayConcatNode using graph! macro");

  // Send configuration (optional for ArrayConcatNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send test data: pairs of arrays to concatenate
  let test_pairs = vec![
    (vec!["a", "b"], vec!["c", "d"]), // ["a", "b"] + ["c", "d"] = ["a", "b", "c", "d"]
    (vec!["x", "y", "z"], vec!["1", "2"]), // ["x", "y", "z"] + ["1", "2"] = ["x", "y", "z", "1", "2"]
  ];

  for (array1, array2) in test_pairs {
    let array1_vec: Vec<Arc<dyn Any + Send + Sync>> = array1
      .iter()
      .map(|s| Arc::new(s.to_string()) as Arc<dyn Any + Send + Sync>)
      .collect();
    let array2_vec: Vec<Arc<dyn Any + Send + Sync>> = array2
      .iter()
      .map(|s| Arc::new(s.to_string()) as Arc<dyn Any + Send + Sync>)
      .collect();

    let _ = in1_tx
      .send(Arc::new(array1_vec) as Arc<dyn Any + Send + Sync>)
      .await;
    let _ = in2_tx
      .send(Arc::new(array2_vec) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with ArrayConcatNode...");
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
      // ArrayConcatNode outputs a concatenated array
      if let Ok(concatenated_arc) = item.clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let concatenated = (**concatenated_arc).to_vec();
        let mut elements = Vec::new();
        for elem in concatenated {
          if let Ok(str_arc) = elem.downcast::<String>() {
            elements.push((**str_arc).to_string());
          }
        }
        println!("  Concatenated array: {:?}", elements);
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

  // Verify behavior: should receive 2 results (2 concatenated arrays)
  if success_count == 2 && error_count == 0 {
    println!("✓ ArrayConcatNode correctly performed array concatenation");
  } else {
    println!(
      "⚠ ArrayConcatNode behavior may be unexpected (successes: {}, errors: {}, expected successes: 2, errors: 0)",
      success_count, error_count
    );
  }

  Ok(())
}
