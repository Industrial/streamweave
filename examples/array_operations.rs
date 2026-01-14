//! # Array Operations Example
//!
//! This example demonstrates how to use array manipulation nodes in StreamWeave.
//! It shows array creation, indexing, filtering, and mapping operations.

use streamweave::graph::node::{InputStreams, Node, OutputStreams};
use streamweave::graph::nodes::array::{ArrayLengthNode, ArrayIndexNode, ArrayFilterNode};
use streamweave::graph::nodes::filter_node::{FilterConfig, FilterFunction, filter_config};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Array Operations Example ===\n");

    // Create array nodes
    let length_node = ArrayLengthNode::new("length".to_string());
    let index_node = ArrayIndexNode::new("index".to_string());
    let filter_node = ArrayFilterNode::new("filter".to_string());

    // Example 1: Array length
    println!("Example 1: Getting length of array [1, 2, 3, 4, 5]");
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

    let array: Vec<Arc<dyn Any + Send + Sync>> = vec![
        Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(5i32) as Arc<dyn Any + Send + Sync>,
    ];
    tx.send(Arc::new(array) as Arc<dyn Any + Send + Sync>).await?;
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

    // Example 2: Array indexing
    println!("Example 2: Getting element at index 2 from array [10, 20, 30, 40]");
    let (tx_arr, rx_arr) = tokio::sync::mpsc::channel(10);
    let (tx_idx, rx_idx) = tokio::sync::mpsc::channel(10);
    let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

    let array: Vec<Arc<dyn Any + Send + Sync>> = vec![
        Arc::new(10i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(20i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(30i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(40i32) as Arc<dyn Any + Send + Sync>,
    ];
    tx_arr.send(Arc::new(array) as Arc<dyn Any + Send + Sync>).await?;
    tx_idx.send(Arc::new(2usize) as Arc<dyn Any + Send + Sync>).await?;
    drop(tx_arr);
    drop(tx_idx);
    drop(tx_config);

    let mut inputs: InputStreams = HashMap::new();
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(rx_arr)));
    inputs.insert("index".to_string(), Box::pin(ReceiverStream::new(rx_idx)));
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

    let outputs = index_node.execute(inputs).await?;
    let mut out_stream = outputs.get("out").unwrap().clone();
    while let Some(result) = out_stream.next().await {
        if let Ok(arc_i32) = result.downcast::<i32>() {
            println!("Result: {}\n", *arc_i32);
        }
    }

    // Example 3: Array filtering (filter even numbers)
    println!("Example 3: Filtering even numbers from [1, 2, 3, 4, 5, 6]");
    let (tx_arr, rx_arr) = tokio::sync::mpsc::channel(10);
    let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

    let array: Vec<Arc<dyn Any + Send + Sync>> = vec![
        Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(3i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(4i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(5i32) as Arc<dyn Any + Send + Sync>,
        Arc::new(6i32) as Arc<dyn Any + Send + Sync>,
    ];
    tx_arr.send(Arc::new(array) as Arc<dyn Any + Send + Sync>).await?;

    // Create filter config that keeps even numbers
    let filter_fn: FilterConfig = filter_config(|item| async move {
        if let Ok(arc_i32) = item.downcast::<i32>() {
            Ok(*arc_i32 % 2 == 0)
        } else {
            Ok(false)
        }
    });
    tx_config.send(Arc::new(filter_fn) as Arc<dyn Any + Send + Sync>).await?;
    drop(tx_arr);
    drop(tx_config);

    let mut inputs: InputStreams = HashMap::new();
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(rx_arr)));
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

    let outputs = filter_node.execute(inputs).await?;
    let mut out_stream = outputs.get("out").unwrap().clone();
    let mut results = Vec::new();
    while let Some(result) = out_stream.next().await {
        if let Ok(arc_i32) = result.downcast::<i32>() {
            results.push(*arc_i32);
        }
    }
    println!("Result: {:?}\n", results);

    println!("All examples completed successfully!");
    Ok(())
}

