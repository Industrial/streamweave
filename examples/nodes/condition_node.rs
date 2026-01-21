use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::Node;
use streamweave::nodes::condition_node::{condition_config, ConditionConfig, ConditionNode};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the ConditionNode directly
    let condition_node = ConditionNode::new("condition".to_string());

    // Create input streams for the ConditionNode
    let (config_tx, config_rx) = mpsc::channel(10);
    let (input_tx, input_rx) = mpsc::channel(10);

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

    println!("✓ ConditionNode created with input streams");

    // Create a configuration that routes positive numbers (including zero) to "true" and negative to "false"
    let condition_config: ConditionConfig = condition_config(|value| async move {
        if let Ok(arc_i32) = value.downcast::<i32>() {
            let num = *arc_i32;
            Ok(num >= 0)
        } else {
            Err("Expected i32".to_string())
        }
    });

    // Send the configuration
    let _ = config_tx
        .send(Arc::new(condition_config) as Arc<dyn Any + Send + Sync>)
        .await;

    // Send test numbers
    let test_numbers = vec![5, -3, 0, 10, -7, 2, -1];
    for num in test_numbers {
        let _ = input_tx
            .send(Arc::new(num) as Arc<dyn Any + Send + Sync>)
            .await;
    }

    println!("✓ Configuration and input data sent to streams");

    // Drop the transmitters to close the input channels (signals EOF to streams)
    // This must happen BEFORE calling execute, so the streams know when input ends
    drop(config_tx);
    drop(input_tx);

    // Execute the ConditionNode
    println!("Executing ConditionNode...");
    let start = std::time::Instant::now();
    let outputs_future = condition_node.execute(inputs);
    let mut outputs = outputs_future
        .await
        .map_err(|e| format!("ConditionNode execution failed: {:?}", e))?;
    println!("✓ ConditionNode execution completed in {:?}", start.elapsed());

    // Read results from the "true" output stream
    println!("Reading results from 'true' output stream...");
    let mut true_count = 0;
    if let Some(mut true_stream) = outputs.remove("true") {
        while let Some(item) = true_stream.next().await {
            if let Ok(num_arc) = item.downcast::<i32>() {
                let num = *num_arc;
                println!("  True: {}", num);
                true_count += 1;
            }
        }
    }

    // Read results from the "false" output stream
    println!("Reading results from 'false' output stream...");
    let mut false_count = 0;
    if let Some(mut false_stream) = outputs.remove("false") {
        while let Some(item) = false_stream.next().await {
            if let Ok(num_arc) = item.downcast::<i32>() {
                let num = *num_arc;
                println!("  False: {}", num);
                false_count += 1;
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

    println!("✓ Received {} numbers routed to 'true' output", true_count);
    println!("✓ Received {} numbers routed to 'false' output", false_count);
    println!("✓ Received {} errors via error channel", error_count);
    println!("✓ Total completed in {:?}", start.elapsed());

    Ok(())
}
