//! Tests for WatermarkInjectorNode.

#![allow(unused_imports, dead_code, clippy::type_complexity)]

use super::WatermarkInjectorNode;
use crate::node::{InputStreams, Node, OutputStreams};
use crate::time::{LogicalTime, StreamMessage, Timestamped};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
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

#[tokio::test]
async fn test_watermark_injector_creation() {
  let node = WatermarkInjectorNode::new("watermark".to_string());
  assert_eq!(node.name(), "watermark");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("in"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_watermark_injector_emits_watermark_on_eos() {
  let node = WatermarkInjectorNode::new("watermark".to_string());
  let (in_tx, inputs) = create_inputs();

  let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

  let mut map = HashMap::new();
  map.insert(
    "event_timestamp".to_string(),
    Arc::new(100i64) as Arc<dyn Any + Send + Sync>,
  );
  map.insert(
    "value".to_string(),
    Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
  );
  in_tx.send(Arc::new(map)).await.unwrap();

  let mut map2 = HashMap::new();
  map2.insert(
    "event_timestamp".to_string(),
    Arc::new(200i64) as Arc<dyn Any + Send + Sync>,
  );
  map2.insert(
    "value".to_string(),
    Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
  );
  in_tx.send(Arc::new(map2)).await.unwrap();

  drop(in_tx);

  let mut out_stream = outputs.remove("out").unwrap();
  let mut results: Vec<Arc<dyn Any + Send + Sync>> = Vec::new();
  while let Some(item) = out_stream.next().await {
    results.push(item);
  }

  assert_eq!(results.len(), 3, "2 Data + 1 Watermark");
  let msg0 = results[0]
    .clone()
    .downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
    .unwrap();
  assert!(msg0.is_data());
  if let StreamMessage::Data(ts) = msg0.as_ref() {
    assert_eq!(ts.time().as_u64(), 100);
  }

  let msg1 = results[1]
    .clone()
    .downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
    .unwrap();
  assert!(msg1.is_data());
  if let StreamMessage::Data(ts) = msg1.as_ref() {
    assert_eq!(ts.time().as_u64(), 200);
  }

  let msg2 = results[2]
    .clone()
    .downcast::<StreamMessage<Arc<dyn Any + Send + Sync>>>()
    .unwrap();
  assert!(msg2.is_watermark());
  if let StreamMessage::Watermark(t) = msg2.as_ref() {
    assert_eq!(t.as_u64(), 200);
  }
}
