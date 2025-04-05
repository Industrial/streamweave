use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, stream};
use std::collections::HashSet;
use std::error::Error as StdError;
use std::fmt;
use std::hash::Hash;
use std::pin::Pin;

#[derive(Debug)]
pub enum HashSetError {
  StreamError(String),
}

impl fmt::Display for HashSetError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      HashSetError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for HashSetError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    None
  }
}

pub struct HashSetProducer<T> {
  data: HashSet<T>,
}

impl<T> HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
{
  pub fn new(data: HashSet<T>) -> Self {
    Self { data }
  }

  pub fn from_iter<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = T>,
  {
    Self {
      data: iter.into_iter().collect(),
    }
  }

  pub fn len(&self) -> usize {
    self.data.len()
  }

  pub fn is_empty(&self) -> bool {
    self.data.is_empty()
  }
}

impl<T> Error for HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
{
  type Error = HashSetError;
}

impl<T> Output for HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, HashSetError>> + Send>>;
}

impl<T> Producer for HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
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
  async fn test_hash_set_producer() {
    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let mut producer = HashSetProducer::new(set);
    let stream = producer.produce();
    let mut result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    result.sort(); // Sort for deterministic comparison

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_hash_set_producer_from_iter() {
    let mut producer = HashSetProducer::from_iter(vec![1, 2, 3, 2, 1]);
    assert_eq!(producer.len(), 3); // Duplicates removed

    let stream = producer.produce();
    let mut result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    result.sort();

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_hash_set_producer_empty() {
    let set: HashSet<i32> = HashSet::new();
    let mut producer = HashSetProducer::new(set);
    assert!(producer.is_empty());

    let stream = producer.produce();
    let result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_hash_set_producer_custom_type() {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct CustomType(String);

    let mut set = HashSet::new();
    set.insert(CustomType("a".to_string()));
    set.insert(CustomType("b".to_string()));

    let mut producer = HashSetProducer::new(set);
    let stream = producer.produce();
    let mut result: Vec<CustomType> = stream.map(|r| r.unwrap()).collect().await;
    result.sort_by_key(|k| k.0.clone());

    assert_eq!(
      result,
      vec![CustomType("a".to_string()), CustomType("b".to_string())]
    );
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut set = HashSet::new();
    set.insert(42);

    let mut producer = HashSetProducer::new(set);

    // First call
    let stream = producer.produce();
    let result1: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec![42]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec![42]);
  }

  #[tokio::test]
  async fn test_from_iter_duplicates() {
    let mut producer = HashSetProducer::from_iter(vec![1, 1, 2, 2, 3, 3]);
    assert_eq!(producer.len(), 3);

    let stream = producer.produce();
    let mut result: Vec<i32> = stream.map(|r| r.unwrap()).collect().await;
    result.sort();

    assert_eq!(result, vec![1, 2, 3]);
  }
}
