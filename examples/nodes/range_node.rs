use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::Node;
use streamweave::nodes::range_node::{RangeConfig, RangeNode};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Create the RangeNode directly
  let range_node = RangeNode::new("range".to_string());

  // Create input streams for the RangeNode
  let (config_tx, config_rx) = mpsc::channel(10);
  let (start_tx, start_rx) = mpsc::channel(10);
  let (end_tx, end_rx) = mpsc::channel(10);
  let (step_tx, step_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "start".to_string(),
    Box::pin(ReceiverStream::new(start_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "end".to_string(),
    Box::pin(ReceiverStream::new(end_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "step".to_string(),
    Box::pin(ReceiverStream::new(step_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );

  println!("✓ RangeNode created with input streams");

  // Configuration is optional for RangeNode
  let config = Arc::new(RangeConfig {});

  // Send the configuration
  let _ = config_tx.send(config as Arc<dyn Any + Send + Sync>).await;

  // Send range parameters: generate numbers from 0 to 10 with step 2
  let _ = start_tx
    .send(Arc::new(0i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = end_tx
    .send(Arc::new(10i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = step_tx
    .send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>)
    .await;

  println!("✓ Configuration and input data sent to streams");

  // Drop the transmitters to close the input channels (signals EOF to streams)
  // This must happen BEFORE calling execute, so the streams know when input ends
  drop(config_tx);
  drop(start_tx);
  drop(end_tx);
  drop(step_tx);

  // Execute the RangeNode
  println!("Executing RangeNode...");
  let start = std::time::Instant::now();
  let outputs_future = range_node.execute(inputs);
  let mut outputs = outputs_future
    .await
    .map_err(|e| format!("RangeNode execution failed: {:?}", e))?;
  println!("✓ RangeNode execution completed in {:?}", start.elapsed());

  // Read results from the "out" output stream
  println!("Reading results from 'out' output stream...");
  let mut number_count = 0;
  if let Some(mut out_stream) = outputs.remove("out") {
    while let Some(item) = out_stream.next().await {
      if let Ok(num_arc) = item.clone().downcast::<i32>() {
        let num = *num_arc;
        println!("  Generated: {}", num);
        number_count += 1;
      } else if let Ok(num_arc) = item.downcast::<f64>() {
        let num = *num_arc;
        println!("  Generated: {}", num);
        number_count += 1;
      }
    }
  }

  // Read errors from the error stream
  println!("Reading errors from error stream...");
  let mut error_count = 0;
  if let Some(mut error_stream) = outputs.remove("error") {
    while let Some(item) = error_stream.next().await {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = &**error_msg;
        println!("  Error: {}", error);
        error_count += 1;
      }
    }
  }

  println!("✓ Received {} numbers via output channel", number_count);
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}
