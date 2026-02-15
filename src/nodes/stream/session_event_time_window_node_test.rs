//! Tests for SessionEventTimeWindowNode.

#![allow(clippy::type_complexity)]

use super::SessionEventTimeWindowNode;
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
async fn test_session_window_creation() {
  let node = SessionEventTimeWindowNode::new("session".to_string(), Duration::from_millis(500));
  assert_eq!(node.name(), "session");
  assert_eq!(node.gap(), Duration::from_millis(500));
}

/// Out-of-order: events 3, 1, 2; gap=500 so all in one session.
#[tokio::test]
async fn test_session_window_out_of_order() {
  let node = SessionEventTimeWindowNode::new("session".to_string(), Duration::from_millis(500));
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
async fn test_session_window_gap_split() {
  // gap=500. Events at 100, 200, 800 (gap>500 from 200) -> two sessions
  let node = SessionEventTimeWindowNode::new("session".to_string(), Duration::from_millis(500));
  let (in_tx, inputs) = create_inputs();
  let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

  in_tx.send(data_at(100, 1)).await.unwrap();
  in_tx.send(data_at(200, 2)).await.unwrap();
  in_tx.send(data_at(800, 3)).await.unwrap(); // gap 600 > 500, new session
  drop(in_tx);

  let mut out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  while let Some(item) = out_stream.next().await {
    results.push(item);
  }
  assert_eq!(results.len(), 2);
}
