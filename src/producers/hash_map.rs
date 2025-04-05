use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, stream};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::hash::Hash;
use std::pin::Pin;

#[derive(Debug)]
pub enum HashMapError {
  StreamError(String),
}

impl fmt::Display for HashMapError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      HashMapError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for HashMapError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct HashMapProducer<K, V> {
  data: HashMap<K, V>,
}

impl<K, V> HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  pub fn new(data: HashMap<K, V>) -> Self {
    Self { data }
  }
}

impl<K, V> Error for HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  type Error = HashMapError;
}

impl<K, V> Output for HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  type Output = (K, V);
  type OutputStream = Pin<Box<dyn Stream<Item = Result<(K, V), HashMapError>> + Send>>;
}

impl<K, V> Producer for HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  fn produce(&mut self) -> Self::OutputStream {
    let data = self.data.clone();
    Box::pin(stream::iter(data.into_iter().map(Ok)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_hash_map_producer() {
    let mut map = HashMap::new();
    map.insert("key1", 1);
    map.insert("key2", 2);
    map.insert("key3", 3);

    let mut producer = HashMapProducer::new(map.clone());
    let stream = producer.produce();
    let mut result: Vec<(&str, i32)> = stream.map(|r| r.unwrap()).collect().await;

    // Sort for deterministic comparison
    result.sort_by_key(|k| k.0);
    assert_eq!(result, vec![("key1", 1), ("key2", 2), ("key3", 3)]);
  }

  #[tokio::test]
  async fn test_hash_map_producer_empty() {
    let map: HashMap<String, i32> = HashMap::new();
    let mut producer = HashMapProducer::new(map);
    let stream = producer.produce();
    let result: Vec<(String, i32)> = stream.map(|r| r.unwrap()).collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_hash_map_producer_custom_types() {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct CustomKey(String);

    #[derive(Clone, Debug, PartialEq)]
    struct CustomValue(i32);

    let mut map = HashMap::new();
    map.insert(CustomKey("a".into()), CustomValue(1));
    map.insert(CustomKey("b".into()), CustomValue(2));

    let mut producer = HashMapProducer::new(map);
    let stream = producer.produce();
    let mut result: Vec<(CustomKey, CustomValue)> = stream.map(|r| r.unwrap()).collect().await;

    // Sort for deterministic comparison
    result.sort_by_key(|k| k.0.0.clone());

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0.0, "a");
    assert_eq!(result[0].1.0, 1);
    assert_eq!(result[1].0.0, "b");
    assert_eq!(result[1].1.0, 2);
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut map = HashMap::new();
    map.insert("test", 42);

    let mut producer = HashMapProducer::new(map);

    // First call
    let stream = producer.produce();
    let result1: Vec<(&str, i32)> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1.len(), 1);
    assert_eq!(result1[0], ("test", 42));

    // Second call
    let stream = producer.produce();
    let result2: Vec<(&str, i32)> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2.len(), 1);
    assert_eq!(result2[0], ("test", 42));
  }
}
