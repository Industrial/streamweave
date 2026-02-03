use std::any::Any;
use std::sync::Arc;
use streamweave::graph;
use streamweave::graph::Graph;
use streamweave::nodes::stream::ZipNode;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (input0_tx, input0_rx) = mpsc::channel(10);
  let (input1_tx, input1_rx) = mpsc::channel(10);
  let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

  // Build the graph using the graph! macro
  let mut graph: Graph = graph! {
    zip: ZipNode::new("zip".to_string(), 2),
    graph.configuration => zip.configuration,
    graph.input0 => zip.in_0,
    graph.input1 => zip.in_1,
    zip.out => graph.output,
    zip.error => graph.error
  };

  // Connect external channels at runtime
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("input0", input0_rx)?;
  graph.connect_input_channel("input1", input1_rx)?;
  graph.connect_output_channel("output", output_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with ZipNode using graph! macro");

  // Send configuration (optional for ZipNode)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send values to both input streams: numbers and letters
  println!("📥 Sending values to input0: 1, 2, 3");
  let numbers = vec![1i32, 2i32, 3i32];
  for num in numbers {
    input0_tx
      .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("📥 Sending values to input1: A, B, C");
  let letters = vec!["A", "B", "C"];
  for letter in letters {
    input1_tx
      .send(Arc::new(letter.to_string()) as Arc<dyn Any + Send + Sync>)
      .await
      .unwrap();
  }

  println!("✓ Test data sent to both input channels");

  // Execute the graph
  println!("Executing graph with ZipNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input0_tx);
  drop(input1_tx);

  // Read results from the output channels
  println!("Reading results from output channels...");
  let mut output_items = Vec::new();
  let mut error_count = 0;

  loop {
    let output_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), output_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(500), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = output_result {
      if let Ok(zipped) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        let zipped_values: Vec<String> = zipped
          .iter()
          .enumerate()
          .map(|(i, arc)| {
            if i == 0 {
              // First element should be i32
              arc
                .clone()
                .downcast::<i32>()
                .map(|v| v.to_string())
                .unwrap_or_else(|_| "unknown".to_string())
            } else {
              // Second element should be String
              arc
                .clone()
                .downcast::<String>()
                .map(|s| (*s).clone())
                .unwrap_or_else(|_| "unknown".to_string())
            }
          })
          .collect();
        println!("  Output zipped pair: {:?}", zipped_values);
        output_items.push(zipped_values);
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
    "✓ Received {} zipped pairs via output channel",
    output_items.len()
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive zipped pairs: [1,"A"], [2,"B"], [3,"C"]
  let expected_output = vec![
    vec!["1".to_string(), "A".to_string()],
    vec!["2".to_string(), "B".to_string()],
    vec!["3".to_string(), "C".to_string()],
  ];

  if output_items == expected_output && error_count == 0 {
    println!(
      "✓ ZipNode correctly zipped streams, producing synchronized pairs: {:?}",
      output_items
    );
  } else {
    println!(
      "⚠ ZipNode behavior may be unexpected (outputs: {:?}, expected: {:?}, errors: {})",
      output_items, expected_output, error_count
    );
  }

  Ok(())
}
