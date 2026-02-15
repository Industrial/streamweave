//! Tests for TumblingProcessingTimeWindowNode.

#![allow(unused_imports, dead_code, unused, clippy::type_complexity)]

use super::TumblingProcessingTimeWindowNode;
use crate::node::{InputStreams, Node, OutputStreams};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

fn create_inputs() -> (mpsc::Sender<Arc<dyn Any + Send + Sync>>, InputStreams) {
    let (_config_tx, config_rx) = mpsc::channel(10);
    let (in_tx, in_rx) = mpsc::channel(10);

    let mut inputs = HashMap::new();
    inputs.insert(
        "configuration".to_string(),
        Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
    );
    inputs.insert(
        "in".to_string(),
        Box::pin(ReceiverStream::new(in_rx)) as crate::node::InputStream,
    );

    (in_tx, inputs)
}

#[tokio::test]
async fn test_tumbling_window_creation() {
    let node = TumblingProcessingTimeWindowNode::new(
        "tumbling".to_string(),
        Duration::from_millis(100),
    );
    assert_eq!(node.name(), "tumbling");
    assert!(node.has_input_port("configuration"));
    assert!(node.has_input_port("in"));
    assert!(node.has_output_port("out"));
    assert!(node.has_output_port("error"));
    assert_eq!(node.window_size(), Duration::from_millis(100));
}

#[tokio::test]
async fn test_tumbling_window_emits_on_stream_end() {
    let node = TumblingProcessingTimeWindowNode::new(
        "tumbling".to_string(),
        Duration::from_millis(500),
    );
    let (in_tx, inputs) = create_inputs();

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

    in_tx.send(Arc::new(1i32)).await.unwrap();
    in_tx.send(Arc::new(2i32)).await.unwrap();
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
    let timeout = tokio::time::sleep(Duration::from_millis(300));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            result = out_stream.next() => {
                if let Some(item) = result {
                    results.push(item);
                } else {
                    break;
                }
            }
            _ = &mut timeout => break,
        }
    }

    assert_eq!(results.len(), 1, "should receive one window on stream end");
    if let Ok(window) = results[0].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
        assert_eq!(window.len(), 2, "window should have 2 items");
    } else {
        panic!("expected Vec output");
    }
}
