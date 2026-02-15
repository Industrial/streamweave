//! Tests for SlidingEventTimeWindowNode.

#![allow(clippy::type_complexity)]

use super::SlidingEventTimeWindowNode;
use crate::node::{InputStreams, Node, OutputStreams};
use crate::time::{LogicalTime, StreamMessage, Timestamped};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

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
  Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Data(
    Timestamped::new(
      Arc::new(payload) as Arc<dyn Any + Send + Sync>,
      LogicalTime::new(t),
    ),
  ))
}

fn watermark(t: u64) -> Arc<dyn Any + Send + Sync> {
  Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(
    LogicalTime::new(t),
  ))
}

#[tokio::test]
async fn test_sliding_window_creation() {
  let node = SlidingEventTimeWindowNode::new(
    "sliding".to_string(),
    Duration::from_millis(1000),
    Duration::from_millis(200),
  );
  assert_eq!(node.name(), "sliding");
  assert_eq!(node.window_size(), Duration::from_millis(1000));
  assert_eq!(node.slide(), Duration::from_millis(200));
}

/// Out-of-order: events 3, 1, 2; all in window [0,1000). Correct results on watermark.
#[tokio::test]
async fn test_sliding_window_out_of_order() {
  let node = SlidingEventTimeWindowNode::new(
    "sliding".to_string(),
    Duration::from_millis(1000),
    Duration::from_millis(1000),
  );
  let (in_tx, inputs) = create_inputs();
  let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

  in_tx.send(data_at(3, 1)).await.unwrap();
  in_tx.send(data_at(1, 2)).await.unwrap();
  in_tx.send(data_at(2, 3)).await.unwrap();
  in_tx.send(watermark(1000)).await.unwrap();
  drop(in_tx);

  let mut out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  while let Some(item) = out_stream.next().await {
    results.push(item);
  }
  assert_eq!(results.len(), 1);
  let w = results[0]
    .clone()
    .downcast::<Vec<Arc<dyn Any + Send + Sync>>>()
    .unwrap();
  assert_eq!(w.len(), 3);
}

#[tokio::test]
async fn test_sliding_window_overlapping() {
  // size=1000, slide=400. Windows: [0,1000), [400,1400), [800,1800), ...
  // T=500 belongs to [0,1000) and [400,1400)
  let node = SlidingEventTimeWindowNode::new(
    "sliding".to_string(),
    Duration::from_millis(1000),
    Duration::from_millis(400),
  );
  let (in_tx, inputs) = create_inputs();
  let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

  in_tx.send(data_at(500, 1)).await.unwrap();
  in_tx.send(watermark(1000)).await.unwrap(); // close [0,1000)
  in_tx.send(watermark(1400)).await.unwrap(); // close [400,1400)
  drop(in_tx);

  let mut out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  while let Some(item) = out_stream.next().await {
    results.push(item);
  }
  // T=500 in [0,1000) and [400,1400) -> two windows
  assert_eq!(results.len(), 2);
}
