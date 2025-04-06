use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;

#[derive(Debug)]
pub struct HashMapConsumer<K, V>
where
  K: Eq + Hash,
{
  map: HashMap<K, V>,
  on_duplicate: DuplicateKeyStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum DuplicateKeyStrategy {
  Keep,    // Keep the first value
  Replace, // Replace with new value
  Error,   // Return an error
}

#[derive(Debug)]
pub enum HashMapConsumerError {
  DuplicateKey(String),
}

impl std::fmt::Display for HashMapConsumerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      HashMapConsumerError::DuplicateKey(k) => write!(f, "Duplicate key encountered: {}", k),
    }
  }
}

impl std::error::Error for HashMapConsumerError {}

impl<K, V> Error for HashMapConsumer<K, V>
where
  K: Eq + Hash,
{
  type Error = HashMapConsumerError;
}

impl<K, V> Input for HashMapConsumer<K, V>
where
  K: Eq + Hash,
{
  type Input = (K, V);
  type InputStream = Pin<Box<dyn futures::Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<K, V> HashMapConsumer<K, V>
where
  K: Eq + Hash,
{
  pub fn new() -> Self {
    Self {
      map: HashMap::new(),
      on_duplicate: DuplicateKeyStrategy::Error,
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      map: HashMap::with_capacity(capacity),
      on_duplicate: DuplicateKeyStrategy::Error,
    }
  }

  pub fn with_strategy(strategy: DuplicateKeyStrategy) -> Self {
    Self {
      map: HashMap::new(),
      on_duplicate: strategy,
    }
  }

  pub fn into_inner(self) -> HashMap<K, V> {
    self.map
  }

  pub fn len(&self) -> usize {
    self.map.len()
  }

  pub fn is_empty(&self) -> bool {
    self.map.is_empty()
  }
}

impl<K, V> Default for HashMapConsumer<K, V>
where
  K: Eq + Hash,
{
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl<K, V> Consumer for HashMapConsumer<K, V>
where
  K: Eq + Hash + Send + std::fmt::Debug,
  V: Send,
{
  async fn consume(&mut self, mut input: Self::InputStream) -> Result<(), Self::Error> {
    while let Some(result) = input.next().await {
      let (key, value) = result?;
      match self.on_duplicate {
        DuplicateKeyStrategy::Keep => {
          self.map.entry(key).or_insert(value);
        }
        DuplicateKeyStrategy::Replace => {
          self.map.insert(key, value);
        }
        DuplicateKeyStrategy::Error => {
          if self.map.contains_key(&key) {
            return Err(HashMapConsumerError::DuplicateKey(format!("{:?}", key)));
          }
          self.map.insert(key, value);
        }
      }
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use tokio;

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = HashMapConsumer::<i32, String>::new();
    let input = stream::empty();
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(consumer.is_empty());
    assert_eq!(consumer.len(), 0);
  }

  #[tokio::test]
  async fn test_basic_insertion() {
    let mut consumer = HashMapConsumer::<i32, String>::new();
    let input = stream::iter(vec![
      Ok((1, "one".to_string())),
      Ok((2, "two".to_string())),
      Ok((3, "three".to_string())),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 3);

    let map = consumer.into_inner();
    assert_eq!(map.get(&1), Some(&"one".to_string()));
    assert_eq!(map.get(&2), Some(&"two".to_string()));
    assert_eq!(map.get(&3), Some(&"three".to_string()));
  }

  #[tokio::test]
  async fn test_duplicate_key_error() {
    let mut consumer = HashMapConsumer::<i32, String>::new();
    let input = stream::iter(vec![
      Ok((1, "one".to_string())),
      Ok((1, "uno".to_string())), // Duplicate key
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      HashMapConsumerError::DuplicateKey(_)
    ));
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 1);

    let map = consumer.into_inner();
    assert_eq!(map.get(&1), Some(&"one".to_string()));
  }

  #[tokio::test]
  async fn test_duplicate_key_keep() {
    let mut consumer = HashMapConsumer::<i32, String>::with_strategy(DuplicateKeyStrategy::Keep);
    let input = stream::iter(vec![
      Ok((1, "one".to_string())),
      Ok((1, "uno".to_string())), // Duplicate key
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 1);

    let map = consumer.into_inner();
    assert_eq!(map.get(&1), Some(&"one".to_string())); // First value kept
  }

  #[tokio::test]
  async fn test_duplicate_key_replace() {
    let mut consumer = HashMapConsumer::<i32, String>::with_strategy(DuplicateKeyStrategy::Replace);
    let input = stream::iter(vec![
      Ok((1, "one".to_string())),
      Ok((1, "uno".to_string())), // Duplicate key
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 1);

    let map = consumer.into_inner();
    assert_eq!(map.get(&1), Some(&"uno".to_string())); // Last value kept
  }

  #[tokio::test]
  async fn test_with_capacity() {
    let mut consumer = HashMapConsumer::<i32, String>::with_capacity(100);
    let input = stream::iter(vec![Ok((1, "one".to_string())), Ok((2, "two".to_string()))]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 2);
  }

  #[tokio::test]
  async fn test_string_keys() {
    let mut consumer = HashMapConsumer::<String, i32>::new();
    let input = stream::iter(vec![Ok(("one".to_string(), 1)), Ok(("two".to_string(), 2))]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 2);

    let map = consumer.into_inner();
    assert_eq!(map.get("one"), Some(&1));
    assert_eq!(map.get("two"), Some(&2));
  }

  #[tokio::test]
  async fn test_complex_types() {
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct ComplexKey {
      id: i32,
      name: String,
    }

    let mut consumer = HashMapConsumer::<ComplexKey, Vec<i32>>::new();
    let input = stream::iter(vec![Ok((
      ComplexKey {
        id: 1,
        name: "test".to_string(),
      },
      vec![1, 2, 3],
    ))]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 1);

    let map = consumer.into_inner();
    let key = ComplexKey {
      id: 1,
      name: "test".to_string(),
    };
    assert_eq!(map.get(&key), Some(&vec![1, 2, 3]));
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut consumer = HashMapConsumer::<i32, String>::new();
    let input = stream::iter(vec![
      Ok((1, "one".to_string())),
      Err(HashMapConsumerError::DuplicateKey("test".to_string())),
      Ok((2, "two".to_string())),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      HashMapConsumerError::DuplicateKey(_)
    ));
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 1);

    let map = consumer.into_inner();
    assert_eq!(map.get(&1), Some(&"one".to_string()));
  }

  #[tokio::test]
  async fn test_default_impl() {
    let consumer = HashMapConsumer::<i32, String>::default();
    assert!(consumer.is_empty());
    assert_eq!(consumer.len(), 0);
  }

  #[tokio::test]
  async fn test_large_dataset() {
    const COUNT: usize = 1000;
    let mut consumer = HashMapConsumer::<i32, String>::with_capacity(COUNT);
    let input = stream::iter((0..COUNT).map(|i| Ok((i as i32, format!("value_{}", i)))));
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), COUNT);

    let map = consumer.into_inner();
    for i in 0..COUNT {
      assert_eq!(map.get(&(i as i32)), Some(&format!("value_{}", i)));
    }
  }
}
