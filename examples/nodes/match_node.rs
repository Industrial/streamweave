use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::Node;
use streamweave::nodes::match_node::{match_exact_string, MatchConfig, MatchNode};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the MatchNode directly
    let match_node = MatchNode::new("match".to_string(), 3);

    // Create input streams for the MatchNode
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

    println!("✓ MatchNode created with input streams");

    // Create a configuration that routes strings to different output ports
    let string_config: MatchConfig = match_exact_string(vec![
        ("error", 0),     // Route to out_0
        ("warning", 1),   // Route to out_1
        ("info", 2),      // Route to out_2
    ]);

    // Send the configuration
    let _ = config_tx
        .send(Arc::new(string_config) as Arc<dyn Any + Send + Sync>)
        .await;

    let test_strings = vec![
        "error",     // Should go to out_0
        "warning",   // Should go to out_1
        "info",      // Should go to out_2
        "debug",     // Should go to default (no match)
        "trace",     // Should go to default (no match)
    ];

    for s in test_strings {
        let _ = input_tx
            .send(Arc::new(s.to_string()) as Arc<dyn Any + Send + Sync>)
            .await;
    }

    // Send a non-string to demonstrate error handling
    let _ = input_tx
        .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
        .await;

    println!("✓ Configuration and input data sent to streams");

    // Drop the transmitters to close the input channels (signals EOF to streams)
    // This must happen BEFORE calling execute, so the streams know when input ends
    drop(config_tx);
    drop(input_tx);

    // Execute the MatchNode
    println!("Executing MatchNode...");
    let start = std::time::Instant::now();
    let outputs_future = match_node.execute(inputs);
    let mut outputs = outputs_future
        .await
        .map_err(|e| format!("MatchNode execution failed: {:?}", e))?;
    println!("✓ MatchNode execution completed in {:?}", start.elapsed());

    // Read results from the output streams
    println!("Reading results from output streams...");
    let mut total_processed = 0;

    // Read from "out_0" stream
    if let Some(mut out0_stream) = outputs.remove("out_0") {
        while let Some(item) = out0_stream.next().await {
            if let Ok(string_arc) = item.downcast::<String>() {
                let value = &**string_arc;
                println!("  out_0: {}", value);
                total_processed += 1;
            }
        }
    }

    // Read from "out_1" stream
    if let Some(mut out1_stream) = outputs.remove("out_1") {
        while let Some(item) = out1_stream.next().await {
            if let Ok(string_arc) = item.downcast::<String>() {
                let value = &**string_arc;
                println!("  out_1: {}", value);
                total_processed += 1;
            }
        }
    }

    // Read from "out_2" stream
    if let Some(mut out2_stream) = outputs.remove("out_2") {
        while let Some(item) = out2_stream.next().await {
            if let Ok(string_arc) = item.downcast::<String>() {
                let value = &**string_arc;
                println!("  out_2: {}", value);
                total_processed += 1;
            }
        }
    }

    // Read from "default" stream
    if let Some(mut default_stream) = outputs.remove("default") {
        while let Some(item) = default_stream.next().await {
            if let Ok(string_arc) = item.downcast::<String>() {
                let value = &**string_arc;
                println!("  default: {}", value);
                total_processed += 1;
            }
        }
    }

    // Read errors from the error stream
    if let Some(mut error_stream) = outputs.remove("error") {
        while let Some(item) = error_stream.next().await {
            if let Ok(error_msg) = item.downcast::<String>() {
                let error = &**error_msg;
                println!("  error: {}", error);
                total_processed += 1;
            }
        }
    }

    println!("✓ Received {} items routed to branches via output ports", total_processed);
    println!("✓ Total completed in {:?}", start.elapsed());

    Ok(())
}
