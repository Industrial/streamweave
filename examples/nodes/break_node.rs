use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::{Node, NodeExecutionError, OutputStreams};
use streamweave::nodes::advanced::break_node::BreakNode;
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create the BreakNode directly
  let break_node = BreakNode::new("break".to_string());

  // Create input streams for the BreakNode
  let (config_tx, config_rx) = mpsc::channel(10);
  let (input_tx, input_rx) = mpsc::channel(10);
  let (signal_tx, signal_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(input_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "signal".to_string(),
    Box::pin(ReceiverStream::new(signal_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );

  println!("✓ BreakNode created with input streams");

  // Send configuration (empty config for break node)
  let _ = config_tx
    .send(Arc::new(()) as Arc<dyn Any + Send + Sync>)
    .await;

  // Create a task to send input data and signals concurrently with node execution
  let input_tx_clone = input_tx.clone();
  let signal_tx_clone = signal_tx.clone();

  tokio::spawn(async move {
    // Send initial input data
    let initial_data = vec!["item1", "item2"];
    for item in initial_data {
      let _ = input_tx_clone
        .send(Arc::new(item.to_string()) as Arc<dyn Any + Send + Sync>)
        .await;
      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Send break signal
    let _ = signal_tx_clone
      .send(Arc::new("BREAK".to_string()) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Send more data after break signal (should not be forwarded)
    let post_break_data = vec!["item3", "item4", "item5"];
    for item in post_break_data {
      let _ = input_tx_clone
        .send(Arc::new(item.to_string()) as Arc<dyn Any + Send + Sync>)
        .await;
      tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
  });

  println!("✓ Configuration sent and concurrent data/signal sender started");

  // Execute the BreakNode
  println!("Executing BreakNode...");
  let start = std::time::Instant::now();
  let outputs_future: Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send>,
  > = break_node.execute(inputs);
  let mut outputs: OutputStreams = outputs_future
    .await
    .map_err(|e| format!("BreakNode execution failed: {:?}", e))?;
  println!("✓ BreakNode execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(input_tx);
  drop(signal_tx);

  // Read results from the "out" output stream
  println!("Reading results from 'out' output stream...");
  let mut output_count = 0;
  if let Some(out_stream) = outputs.remove("out") {
    let mut out_stream: Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>> = out_stream;
    while let Some(item) = out_stream.next().await {
      if let Ok(item_arc) = item.downcast::<String>() {
        let item_str = &**item_arc;
        println!("  Output: {}", item_str);
        output_count += 1;
      }
    }
  }

  // Read errors from the error stream
  println!("Reading errors from error stream...");
  let mut error_count = 0;
  if let Some(error_stream) = outputs.remove("error") {
    let mut error_stream: Pin<Box<dyn Stream<Item = Arc<dyn Any + Send + Sync>> + Send>> =
      error_stream;
    while let Some(item) = error_stream.next().await {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = &**error_msg;
        println!("  Error: {}", error);
        error_count += 1;
      }
    }
  }

  println!("✓ Received {} items via output stream", output_count);
  println!("✓ Received {} errors via error stream", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive only the items sent before break signal
  if output_count == 2 {
    println!("✓ BreakNode correctly stopped forwarding after break signal");
  } else {
    println!(
      "⚠ BreakNode behavior may be unexpected (received {} items, expected 2)",
      output_count
    );
  }

  Ok(())
}
