//! Tests for ToDifferentialNode.

#![allow(clippy::type_complexity)]

use super::ToDifferentialNode;
use crate::node::{Node, OutputStreams};
use crate::time::{DifferentialStreamMessage, LogicalTime, StreamMessage, Timestamped};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

#[tokio::test]
async fn test_to_differential_plain_items() {
    let node = ToDifferentialNode::new("to_diff".to_string());
    let (_config_tx, config_rx) = mpsc::channel(10);
    let (in_tx, in_rx) = mpsc::channel(10);
    let mut inputs = HashMap::new();
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(config_rx)) as _);
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(in_rx)) as _);

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();
    in_tx.send(Arc::new(1i32) as Arc<dyn Any + Send + Sync>).await.unwrap();
    in_tx.send(Arc::new(2i32) as Arc<dyn Any + Send + Sync>).await.unwrap();
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let r1 = out_stream.next().await.unwrap();
    let r2 = out_stream.next().await.unwrap();
    assert!(out_stream.next().await.is_none());

    let msg1 = r1.downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    let msg2 = r2.downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    match &*msg1 {
        DifferentialStreamMessage::Data(e) => {
            assert_eq!(e.diff(), 1);
            assert_eq!(e.time(), LogicalTime::new(0));
            assert!(e.payload().clone().downcast::<i32>().is_ok());
        }
        _ => panic!("expected Data"),
    }
    match &*msg2 {
        DifferentialStreamMessage::Data(e) => {
            assert_eq!(e.diff(), 1);
            assert_eq!(e.time(), LogicalTime::new(1));
        }
        _ => panic!("expected Data"),
    }
}

#[tokio::test]
async fn test_to_differential_timestamped_and_watermark() {
    let node = ToDifferentialNode::new("to_diff".to_string());
    let (_config_tx, config_rx) = mpsc::channel(10);
    let (in_tx, in_rx) = mpsc::channel(10);
    let mut inputs = HashMap::new();
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(config_rx)) as _);
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(in_rx)) as _);

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();
    let ts = Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Data(Timestamped::new(
        Arc::new(42i32) as Arc<dyn Any + Send + Sync>,
        LogicalTime::new(100),
    ))) as Arc<dyn Any + Send + Sync>;
    in_tx.send(ts).await.unwrap();
    let wm = Arc::new(StreamMessage::<Arc<dyn Any + Send + Sync>>::Watermark(LogicalTime::new(200)))
        as Arc<dyn Any + Send + Sync>;
    in_tx.send(wm).await.unwrap();
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let r1 = out_stream.next().await.unwrap();
    let r2 = out_stream.next().await.unwrap();
    assert!(out_stream.next().await.is_none());

    let msg1 = r1.downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    match &*msg1 {
        DifferentialStreamMessage::Data(e) => {
            assert_eq!(e.time(), LogicalTime::new(100));
            assert_eq!(e.diff(), 1);
        }
        _ => panic!("expected Data"),
    }
    let msg2 = r2.downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    match &*msg2 {
        DifferentialStreamMessage::Watermark(t) => assert_eq!(*t, LogicalTime::new(200)),
        _ => panic!("expected Watermark"),
    }
}
