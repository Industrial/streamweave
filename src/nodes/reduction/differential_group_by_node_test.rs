//! Tests for DifferentialGroupByNode.

#![allow(clippy::type_complexity)]

use super::DifferentialGroupByNode;
use crate::node::{Node, OutputStreams};
use crate::nodes::reduction::{group_by_config, GroupByConfigWrapper};
use crate::time::{DifferentialElement, DifferentialStreamMessage, LogicalTime};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

#[tokio::test]
async fn test_differential_group_by_count() {
    let node = DifferentialGroupByNode::new("diff_group".to_string());
    let (_config_tx, config_rx) = mpsc::channel(10);
    let (in_tx, in_rx) = mpsc::channel(10);
    let (key_tx, key_rx) = mpsc::channel(10);
    let mut inputs = HashMap::new();
    inputs.insert("configuration".to_string(), Box::pin(ReceiverStream::new(config_rx)) as _);
    inputs.insert("in".to_string(), Box::pin(ReceiverStream::new(in_rx)) as _);
    inputs.insert("key_function".to_string(), Box::pin(ReceiverStream::new(key_rx)) as _);

    let key_fn = group_by_config(|v| async move {
        v.downcast::<i32>()
            .map(|arc| format!("{}", *arc))
            .map_err(|_| "expected i32".to_string())
    });
    key_tx
        .send(Arc::new(GroupByConfigWrapper::new(key_fn)) as Arc<dyn Any + Send + Sync>)
        .await
        .unwrap();
    drop(key_tx);

    let mut outputs: OutputStreams = node.execute(inputs).await.unwrap();

    // Insert 1, 1, 2 -> counts: a=2, b=1 (if we use identity as key - actually we use the number as key)
    // Key extractor: i32 -> to_string. So 1 -> "1", 2 -> "2"
    in_tx
        .send(
            Arc::new(DifferentialStreamMessage::Data(DifferentialElement::insert(
                Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
                LogicalTime::new(0),
            ))) as Arc<dyn Any + Send + Sync>,
        )
        .await
        .unwrap();
    in_tx
        .send(
            Arc::new(DifferentialStreamMessage::Data(DifferentialElement::insert(
                Arc::new(1i32) as Arc<dyn Any + Send + Sync>,
                LogicalTime::new(1),
            ))) as Arc<dyn Any + Send + Sync>,
        )
        .await
        .unwrap();
    in_tx
        .send(
            Arc::new(DifferentialStreamMessage::Data(DifferentialElement::insert(
                Arc::new(2i32) as Arc<dyn Any + Send + Sync>,
                LogicalTime::new(2),
            ))) as Arc<dyn Any + Send + Sync>,
        )
        .await
        .unwrap();
    drop(in_tx);

    let mut out_stream = outputs.remove("out").unwrap();
    let mut results = Vec::new();
    while let Some(item) = out_stream.next().await {
        results.push(item);
    }

    assert_eq!(results.len(), 3);
    let msg0 = results[0].clone().downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    let msg1 = results[1].clone().downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    let msg2 = results[2].clone().downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>().unwrap();
    if let DifferentialStreamMessage::Data(e) = &*msg0 {
        let kv = e.payload().clone().downcast::<(String, i64)>().unwrap();
        assert_eq!(kv.0, "1");
        assert_eq!(kv.1, 1);
        assert_eq!(e.diff(), 1);
    }
    if let DifferentialStreamMessage::Data(e) = &*msg1 {
        let kv = e.payload().clone().downcast::<(String, i64)>().unwrap();
        assert_eq!(kv.0, "1");
        assert_eq!(kv.1, 2);
        assert_eq!(e.diff(), 1);
    }
    if let DifferentialStreamMessage::Data(e) = &*msg2 {
        let kv = e.payload().clone().downcast::<(String, i64)>().unwrap();
        assert_eq!(kv.0, "2");
        assert_eq!(kv.1, 1);
        assert_eq!(e.diff(), 1);
    }
}
