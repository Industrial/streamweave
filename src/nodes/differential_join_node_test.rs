//! Tests for DifferentialJoinNode

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::differential_join_node::DifferentialJoinNode;
use crate::nodes::join_node::{JoinStrategy, join_config};
use crate::time::{DifferentialElement, DifferentialStreamMessage, LogicalTime};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

fn create_input_streams() -> (TestSender, TestSender, TestSender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (left_tx, left_rx) = mpsc::channel(10);
  let (right_tx, right_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "left".to_string(),
    Box::pin(ReceiverStream::new(left_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "right".to_string(),
    Box::pin(ReceiverStream::new(right_rx)) as crate::node::InputStream,
  );

  (config_tx, left_tx, right_tx, inputs)
}

fn to_differential(
  payload: Arc<dyn Any + Send + Sync>,
  time: u64,
  diff: i64,
) -> Arc<dyn Any + Send + Sync> {
  let elem = DifferentialElement::new(payload, LogicalTime::new(time), diff);
  Arc::new(DifferentialStreamMessage::Data(elem)) as Arc<dyn Any + Send + Sync>
}

#[tokio::test]
async fn test_differential_join_node_creation() {
  let node = DifferentialJoinNode::new("test_diff_join".to_string());
  assert_eq!(node.name(), "test_diff_join");
  assert!(node.has_input_port("configuration"));
  assert!(node.has_input_port("left"));
  assert!(node.has_input_port("right"));
  assert!(node.has_output_port("out"));
  assert!(node.has_output_port("error"));
}

#[tokio::test]
async fn test_differential_join_inner_join() {
  let node = DifferentialJoinNode::new("test_diff_join".to_string());
  let (config_tx, left_tx, right_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  let key_fn = |item: Arc<dyn Any + Send + Sync>| {
    Box::pin(async move {
      if let Ok(arc_map) = item.downcast::<HashMap<String, i32>>() {
        if let Some(id) = arc_map.get("id") {
          Ok(id.to_string())
        } else {
          Err("Missing 'id' key".to_string())
        }
      } else {
        Err("Expected HashMap<String, i32>".to_string())
      }
    })
  };

  let config = join_config(
    JoinStrategy::Inner,
    key_fn,
    key_fn,
    |left: Arc<dyn Any + Send + Sync>, right: Option<Arc<dyn Any + Send + Sync>>| {
      Box::pin(async move {
        let mut result = HashMap::new();
        result.insert("left".to_string(), left);
        if let Some(r) = right {
          result.insert("right".to_string(), r);
        }
        Ok(Arc::new(result) as Arc<dyn Any + Send + Sync>)
      })
    },
  );

  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  let mut left_item = HashMap::new();
  left_item.insert("id".to_string(), 1);
  left_item.insert("name".to_string(), 10);
  let _ = left_tx
    .send(to_differential(Arc::new(left_item), 1, 1))
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let mut right_item = HashMap::new();
  right_item.insert("id".to_string(), 1);
  right_item.insert("value".to_string(), 100);
  let _ = right_tx
    .send(to_differential(Arc::new(right_item), 1, 1))
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  let timeout = tokio::time::sleep(tokio::time::Duration::from_millis(200));
  tokio::pin!(timeout);
  loop {
    tokio::select! {
        _ = &mut timeout => break,
        Some(item) = stream.next() => {
            if let Ok(msg) = item.clone().downcast::<DifferentialStreamMessage<Arc<dyn Any + Send + Sync>>>()
                && let Some(elem) = msg.data()
            {
                results.push((elem.payload().clone(), elem.time(), elem.diff()));
            }
        }
    }
  }
  assert_eq!(results.len(), 1);
  let (payload, _time, diff) = &results[0];
  assert_eq!(*diff, 1);
  let joined = payload
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    .unwrap();
  assert!(joined.contains_key("left"));
  assert!(joined.contains_key("right"));
}
