//! # Complex Composition Example
//!
//! This example demonstrates how to compose multiple nodes together to create
//! complex data processing pipelines. It shows:
//! - Chaining multiple transformations
//! - Using different node categories together
//! - Error handling across the pipeline

use streamweave::graph::node::{InputStreams, Node, OutputStreams};
use streamweave::graph::nodes::arithmetic::AddNode;
use streamweave::graph::nodes::string::{StringConcatNode, StringLengthNode};
use streamweave::graph::nodes::array::{ArrayLengthNode, ArrayFilterNode};
use streamweave::graph::nodes::filter_node::{FilterConfig, filter_config};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("=== Complex Composition Example ===\n");

    // Example: Process an array of numbers
    // 1. Filter even numbers
    // 2. Calculate sum using arithmetic operations
    // 3. Convert to string
    // 4. Get string length

    println!("Example: Filter even numbers, sum them, convert to string, get length");
    
    // Step 1: Filter even numbers from array
    let filter_node = ArrayFilterNode::new("filter".to_string());
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
    let mut filtered_stream = outputs.get("out").unwrap().clone();
    
    // Collect filtered results
    let mut filtered = Vec::new();
    while let Some(result) = filtered_stream.next().await {
        if let Ok(arc_i32) = result.downcast::<i32>() {
            filtered.push(*arc_i32);
        }
    }
    
    println!("Filtered even numbers: {:?}", filtered);
    
    // Step 2: Sum the filtered numbers
    if filtered.len() >= 2 {
        let add_node = AddNode::new("add".to_string());
        let (tx1, rx1) = tokio::sync::mpsc::channel(10);
        let (tx2, rx2) = tokio::sync::mpsc::channel(10);
        let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

        tx1.send(Arc::new(filtered[0]) as Arc<dyn Any + Send + Sync>).await?;
        tx2.send(Arc::new(filtered[1]) as Arc<dyn Any + Send + Sync>).await?;
        drop(tx1);
        drop(tx2);
        drop(tx_config);

        let mut inputs: InputStreams = HashMap::new();
        inputs.insert("in1".to_string(), Box::pin(ReceiverStream::new(rx1)));
        inputs.insert("in2".to_string(), Box::pin(ReceiverStream::new(rx2)));
        inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

        let outputs = add_node.execute(inputs).await?;
        let mut sum_stream = outputs.get("out").unwrap().clone();
        
        let mut sum = 0i32;
        while let Some(result) = sum_stream.next().await {
            if let Ok(arc_i32) = result.downcast::<i32>() {
                sum = *arc_i32;
            }
        }
        
        // Continue summing with remaining numbers
        for &num in &filtered[2..] {
            let add_node = AddNode::new("add".to_string());
            let (tx1, rx1) = tokio::sync::mpsc::channel(10);
            let (tx2, rx2) = tokio::sync::mpsc::channel(10);
            let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

            tx1.send(Arc::new(sum) as Arc<dyn Any + Send + Sync>).await?;
            tx2.send(Arc::new(num) as Arc<dyn Any + Send + Sync>).await?;
            drop(tx1);
            drop(tx2);
            drop(tx_config);

            let mut inputs: InputStreams = HashMap::new();
            inputs.insert("in1".to_string(), Box::pin(ReceiverStream::new(rx1)));
            inputs.insert("in2".to_string(), Box::pin(ReceiverStream::new(rx2)));
            inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

            let outputs = add_node.execute(inputs).await?;
            let mut sum_stream = outputs.get("out").unwrap().clone();
            
            while let Some(result) = sum_stream.next().await {
                if let Ok(arc_i32) = result.downcast::<i32>() {
                    sum = *arc_i32;
                }
            }
        }
        
        println!("Sum: {}", sum);
        
        // Step 3: Convert sum to string
        let sum_str = sum.to_string();
        println!("Sum as string: '{}'", sum_str);
        
        // Step 4: Get string length
        let length_node = StringLengthNode::new("length".to_string());
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let (tx_config, rx_config) = tokio::sync::mpsc::channel(10);

        tx.send(Arc::new(sum_str) as Arc<dyn Any + Send + Sync>).await?;
        drop(tx);
        drop(tx_config);

        let mut inputs: InputStreams = HashMap::new();
        inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(rx)));
        inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(rx_config)));

        let outputs = length_node.execute(inputs).await?;
        let mut length_stream = outputs.get("out").unwrap().clone();
        
        while let Some(result) = length_stream.next().await {
            if let Ok(arc_usize) = result.downcast::<usize>() {
                println!("String length: {}\n", *arc_usize);
            }
        }
    }

    println!("Complex composition example completed successfully!");
    Ok(())
}

