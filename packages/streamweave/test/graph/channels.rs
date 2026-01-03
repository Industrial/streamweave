//! Tests for graph channels module

use bytes::Bytes;
use std::sync::Arc;
use streamweave::graph::channels::{ChannelItem, TypeErasedReceiver, TypeErasedSender};

#[test]
fn test_channel_item_as_bytes() {
  let item = ChannelItem::Bytes(Bytes::from("test"));
  assert_eq!(item.as_bytes(), Some(&Bytes::from("test")));

  let item = ChannelItem::Arc(Arc::new(42i32));
  assert_eq!(item.as_bytes(), None);
}

#[tokio::test]
async fn test_type_erased_channels() {
  let (sender, mut receiver): (TypeErasedSender, TypeErasedReceiver) =
    tokio::sync::mpsc::channel(10);

  let item = ChannelItem::Bytes(Bytes::from("test"));
  sender.send(item).await.unwrap();

  let received = receiver.recv().await.unwrap();
  assert!(matches!(received, ChannelItem::Bytes(_)));
}

#[test]
fn test_channel_item_clone() {
  let item1 = ChannelItem::Bytes(Bytes::from("test"));
  let item2 = item1.clone();

  assert_eq!(item1.as_bytes(), item2.as_bytes());
}

#[test]
fn test_channel_item_debug() {
  let item = ChannelItem::Bytes(Bytes::from("test"));
  // Should compile - implements Debug
  let _ = format!("{:?}", item);
  assert!(true);
}
