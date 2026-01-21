use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::Node;
use streamweave::nodes::error_branch_node::{ErrorBranchConfig, ErrorBranchNode};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the ErrorBranchNode directly
    let error_branch_node = ErrorBranchNode::new("error_branch".to_string());

    // Create input streams for the ErrorBranchNode
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

    println!("✓ ErrorBranchNode created with input streams");

    // Configuration is optional for ErrorBranchNode
    let config = Arc::new(ErrorBranchConfig {});

    // Send the configuration
    let _ = config_tx
        .send(config as Arc<dyn Any + Send + Sync>)
        .await;

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
    }

    // Also send a non-Result type to demonstrate error handling
    let _ = input_tx
        .send(Arc::new("not_a_result".to_string()) as Arc<dyn Any + Send + Sync>)
        .await;

    println!("✓ Configuration and input data sent to streams");

    // Drop the transmitters to close the input channels (signals EOF to streams)
    // This must happen BEFORE calling execute, so the streams know when input ends
    drop(config_tx);
    drop(input_tx);

    // Execute the ErrorBranchNode
    println!("Executing ErrorBranchNode...");
    let start = std::time::Instant::now();
    let outputs_future = error_branch_node.execute(inputs);
    let mut outputs = outputs_future
        .await
        .map_err(|e| format!("ErrorBranchNode execution failed: {:?}", e))?;
    println!("✓ ErrorBranchNode execution completed in {:?}", start.elapsed());

    // Read results from the "success" output stream
    println!("Reading results from 'success' output stream...");
    let mut success_count = 0;
    if let Some(mut success_stream) = outputs.remove("success") {
        while let Some(item) = success_stream.next().await {
            if let Ok(string_arc) = item.clone().downcast::<String>() {
                let value = &**string_arc;
                println!("  Success: {}", value);
                success_count += 1;
            } else if let Ok(int_arc) = item.downcast::<i32>() {
                let value = *int_arc;
                println!("  Success: {}", value);
                success_count += 1;
            }
        }
    }

    // Read results from the "error" output stream
    println!("Reading results from 'error' output stream...");
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

    println!("✓ Received {} successful results via success channel", success_count);
    println!("✓ Received {} errors via error channel", error_count);
    println!("✓ Total completed in {:?}", start.elapsed());

    Ok(())
}
