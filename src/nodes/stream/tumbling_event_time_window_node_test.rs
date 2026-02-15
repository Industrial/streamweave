//! Tests for TumblingEventTimeWindowNode.

#![allow(unused_imports, dead_code, clippy::type_complexity)]

use super::TumblingEventTimeWindowNode;
use crate::node::{InputStreams, Node, OutputStreams};
use crate::time::{LogicalTime, StreamMessage, Timestamped};
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

fn data_at(t: u64, payload: i32) -> Arc<dyn Any + Send + Sync> {
    Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Data(Timestamped::new(
        Arc::new(payload) as Arc<dyn Any + Send + Sync>,
        LogicalTime::new(t),
    )))
}

fn watermark(t: u64) -> Arc<dyn Any + Send + Sync> {
    Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(LogicalTime::new(t)))
}

#[tokio::test]
async fn test_event_time_window_creation() {
    let node = TumblingEventTimeWindowNode::new(
        "event_window".to_string(),
        Duration::from_secs(3600),
    );
    assert_eq!(node.name(), "event_window");
    assert!(node.has_input_port("configuration"));
    assert!(node.has_input_port("in"));
    assert!(node.has_output_port("out"));
    assert!(node.has_output_port("error"));
    assert_eq!(node.window_size(), Duration::from_secs(3600));
}

#[tokio::test]
async fn test_event_time_window_close_on_watermark() {
    let node = TumblingEventTimeWindowNode::new(
        "event_window".to_string(),
        Duration::from_millis(1000),
    );
    let (in_tx, inputs) = create_inputs();

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

    // Times: 0-999 -> window [0,1000), 1000-1999 -> [1000,2000)
    in_tx.send(data_at(100, 1)).await.unwrap();
    in_tx.send(data_at(500, 2)).await.unwrap();
    in_tx.send(watermark(1000)).await.unwrap(); // close [0,1000)
    in_tx.send(data_at(1100, 3)).await.unwrap();
    in_tx.send(watermark(2000)).await.unwrap(); // close [1000,2000)
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
    while let Some(item) = out_stream.next().await {
        results.push(item);
    }

    assert_eq!(results.len(), 2, "should receive two windows");
    let w0 = results[0].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>().unwrap();
    let w1 = results[1].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>().unwrap();
    assert_eq!(w0.len(), 2, "first window: 2 items (times 100, 500)");
    assert_eq!(w1.len(), 1, "second window: 1 item (time 1100)");
}

#[tokio::test]
async fn test_event_time_window_late_data_dropped() {
    let node = TumblingEventTimeWindowNode::new(
        "event_window".to_string(),
        Duration::from_millis(1000),
    );
    let (in_tx, inputs) = create_inputs();

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

    in_tx.send(data_at(100, 1)).await.unwrap();
    in_tx.send(watermark(1000)).await.unwrap(); // close [0,1000)
    in_tx.send(data_at(50, 2)).await.unwrap();  // late: window already closed
    in_tx.send(data_at(1100, 3)).await.unwrap();
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
    while let Some(item) = out_stream.next().await {
        results.push(item);
    }

    assert_eq!(results.len(), 2, "two windows: [0,1000) and [1000,2000)");
    let w0 = results[0].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>().unwrap();
    let w1 = results[1].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>().unwrap();
    assert_eq!(w0.len(), 1, "first window: only item 1 (item 2 late, dropped)");
    assert_eq!(w1.len(), 1, "second window: item 3");
}

#[tokio::test]
async fn test_event_time_window_eos_flushes() {
    let node = TumblingEventTimeWindowNode::new(
        "event_window".to_string(),
        Duration::from_millis(1000),
    );
    let (in_tx, inputs) = create_inputs();

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

    in_tx.send(data_at(100, 1)).await.unwrap();
    in_tx.send(data_at(500, 2)).await.unwrap();
    // no watermark; drop sender -> EOS flushes remaining windows
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
    while let Some(item) = out_stream.next().await {
        results.push(item);
    }

    assert_eq!(results.len(), 1, "EOS should flush one window");
    let w = results[0].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>().unwrap();
    assert_eq!(w.len(), 2);
}

#[tokio::test]
async fn test_event_time_window_timestamped_only_input() {
    let node = TumblingEventTimeWindowNode::new(
        "event_window".to_string(),
        Duration::from_millis(1000),
    );
    let (in_tx, inputs) = create_inputs();

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

    // Timestamped (no StreamMessage) - no watermarks, EOS flushes
    let ts1 = Arc::new(Timestamped::new(
        Arc::new(10i32) as Arc<dyn Any + Send + Sync>,
        LogicalTime::new(200),
    ));
    let ts2 = Arc::new(Timestamped::new(
        Arc::new(20i32) as Arc<dyn Any + Send + Sync>,
        LogicalTime::new(600),
    ));
    in_tx.send(ts1).await.unwrap();
    in_tx.send(ts2).await.unwrap();
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
    while let Some(item) = out_stream.next().await {
        results.push(item);
    }

    assert_eq!(results.len(), 1);
    let w = results[0].clone().downcast::<Vec<Arc<dyn Any + Send + Sync>>>().unwrap();
    assert_eq!(w.len(), 2);
}
