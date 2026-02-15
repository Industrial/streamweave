//! Tests for MemoizingMapNode.

use crate::node::{InputStreams, Node};
use crate::nodes::common::TestSender;
use crate::nodes::map_node::{map_config, MapConfig};
use crate::nodes::memoizing_map_node::{HashKeyExtractor, MemoizingMapNode};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

fn create_input_streams() -> (TestSender, TestSender, InputStreams) {
  let (config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as crate::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as crate::node::InputStream,
  );

  (config_tx, data_tx, inputs)
}

#[tokio::test]
async fn test_memoizing_map_node_identity_cache() {
  let node = MemoizingMapNode::new("memo".to_string());
  let config: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let item = Arc::new(5i32) as Arc<dyn Any + Send + Sync>;
  let _ = data_tx.send(item.clone()).await;
  let _ = data_tx.send(item.clone()).await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  for _ in 0..2 {
    if let Some(item) = stream.next().await {
      if let Ok(arc_i32) = item.downcast::<i32>() {
        results.push(*arc_i32);
      }
    }
  }

  assert_eq!(results, vec![10, 10]);
}

#[tokio::test]
async fn test_memoizing_map_node_value_cache() {
  let node =
    MemoizingMapNode::with_key_extractor("memo".to_string(), Arc::new(HashKeyExtractor));
  let config: MapConfig = map_config(|value| async move {
    if let Ok(arc_i32) = value.downcast::<i32>() {
      Ok(Arc::new(*arc_i32 * 2) as Arc<dyn Any + Send + Sync>)
    } else {
      Err("Expected i32".to_string())
    }
  });

  let (config_tx, data_tx, inputs) = create_input_streams();
  let outputs_future = node.execute(inputs);
  let mut outputs = outputs_future.await.unwrap();

  let _ = config_tx
    .send(Arc::new(config) as Arc<dyn Any + Send + Sync>)
    .await;
  tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;
  let _ = data_tx
    .send(Arc::new(5i32) as Arc<dyn Any + Send + Sync>)
    .await;

  let out_stream = outputs.remove("out").unwrap();
  let mut results = Vec::new();
  let mut stream = out_stream;
  for _ in 0..2 {
    if let Some(item) = stream.next().await {
      if let Ok(arc_i32) = item.downcast::<i32>() {
        results.push(*arc_i32);
      }
    }
  }

  assert_eq!(results, vec![10, 10]);
}
