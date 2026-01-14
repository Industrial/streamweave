//! # String Operations Example
//!
//! This example demonstrates how to use string manipulation nodes in StreamWeave.
//! It shows string concatenation, length calculation, and other string operations.

use streamweave::graph::node::{InputStreams, Node, OutputStreams};
use streamweave::graph::nodes::string::{StringConcatNode, StringLengthNode, StringSliceNode};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== String Operations Example ===\n");

    // Create string nodes
    let concat_node = StringConcatNode::new("concat".to_string());
    let length_node = StringLengthNode::new("length".to_string());
    let slice_node = StringSliceNode::new("slice".to_string());

    // Example 1: String concatenation
    println!("Example 1: Concatenating 'Hello' + ' World'");
    let (tx1, rx1) = tokio::sync::mpsc::channel(10);
    let (tx2, rx2) = tokio::sync::mpsc::channel(10);
    let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

    tx1.send(Arc::new("Hello".to_string()) as Arc<dyn Any + Send + Sync>).await?;
    tx2.send(Arc::new(" World".to_string()) as Arc<dyn Any + Send + Sync>).await?;
    drop(tx1);
    drop(tx2);
    drop(tx_config);

    let mut inputs: InputStreams = HashMap::new();
    inputs.insert("in1".to_string(), Box::pin(ReceiverStream::new(rx1)));
    inputs.insert("in2".to_string(), Box::pin(ReceiverStream::new(rx2)));
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

    let outputs = concat_node.execute(inputs).await?;
    let mut out_stream = outputs.get("out").unwrap().clone();
    while let Some(result) = out_stream.next().await {
        if let Ok(arc_str) = result.downcast::<String>() {
            println!("Result: '{}'\n", *arc_str);
        }
    }

    // Example 2: String length
    println!("Example 2: Getting length of 'StreamWeave'");
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

    tx.send(Arc::new("StreamWeave".to_string()) as Arc<dyn Any + Send + Sync>).await?;
    drop(tx);
    drop(tx_config);

    let mut inputs: InputStreams = HashMap::new();
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(rx)));
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

    let outputs = length_node.execute(inputs).await?;
    let mut out_stream = outputs.get("out").unwrap().clone();
    while let Some(result) = out_stream.next().await {
        if let Ok(arc_usize) = result.downcast::<usize>() {
            println!("Result: {}\n", *arc_usize);
        }
    }

    // Example 3: String slicing
    println!("Example 3: Slicing 'Hello World' from index 0 to 5");
    let (tx_str, rx_str) = tokio::sync::mpsc::channel(10);
    let (tx_start, rx_start) = tokio::sync::mpsc::channel(10);
    let (tx_end, rx_end) = tokio::sync::mpsc::channel(10);
    let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

    tx_str.send(Arc::new("Hello World".to_string()) as Arc<dyn Any + Send + Sync>).await?;
    tx_start.send(Arc::new(0usize) as Arc<dyn Any + Send + Sync>).await?;
    tx_end.send(Arc::new(5usize) as Arc<dyn Any + Send + Sync>).await?;
    drop(tx_str);
    drop(tx_start);
    drop(tx_end);
    drop(tx_config);

    let mut inputs: InputStreams = HashMap::new();
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(rx_str)));
    inputs.insert("start".to_string(), Box::pin(ReceiverStream::new(rx_start)));
    inputs.insert("end".to_string(), Box::pin(ReceiverStream::new(rx_end)));
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

    let outputs = slice_node.execute(inputs).await?;
    let mut out_stream = outputs.get("out").unwrap().clone();
    while let Some(result) = out_stream.next().await {
        if let Ok(arc_str) = result.downcast::<String>() {
            println!("Result: '{}'\n", *arc_str);
        }
    }

    println!("All examples completed successfully!");
    Ok(())
}

