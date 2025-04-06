use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::StreamExt;
use std::collections::HashSet;
use std::hash::Hash;
use std::pin::Pin;

#[derive(Debug)]
pub struct HashSetConsumer<T>
where
  T: Eq + Hash + Clone,
{
  set: HashSet<T>,
  on_duplicate: DuplicateStrategy,
}

#[derive(Debug, Clone, Copy)]
pub enum DuplicateStrategy {
  Ignore, // Skip duplicates silently
  Error,  // Return an error on duplicates
}

#[derive(Debug)]
pub enum HashSetConsumerError {
  DuplicateItem(String),
}

impl std::fmt::Display for HashSetConsumerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      HashSetConsumerError::DuplicateItem(item) => {
        write!(f, "Duplicate item encountered: {}", item)
      }
    }
  }
}

impl std::error::Error for HashSetConsumerError {}

impl<T> Error for HashSetConsumer<T>
where
  T: Eq + Hash + Clone,
{
  type Error = HashSetConsumerError;
}

impl<T> Input for HashSetConsumer<T>
where
  T: Eq + Hash + Clone,
{
  type Input = T;
  type InputStream = Pin<Box<dyn futures::Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl<T> HashSetConsumer<T>
where
  T: Eq + Hash + Clone,
{
  pub fn new() -> Self {
    Self {
      set: HashSet::new(),
      on_duplicate: DuplicateStrategy::Error,
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      set: HashSet::with_capacity(capacity),
      on_duplicate: DuplicateStrategy::Error,
    }
  }

  pub fn with_strategy(strategy: DuplicateStrategy) -> Self {
    Self {
      set: HashSet::new(),
      on_duplicate: strategy,
    }
  }

  pub fn into_inner(self) -> HashSet<T> {
    self.set
  }

  pub fn len(&self) -> usize {
    self.set.len()
  }

  pub fn is_empty(&self) -> bool {
    self.set.is_empty()
  }
}

impl<T> Default for HashSetConsumer<T>
where
  T: Eq + Hash + Clone,
{
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl<T> Consumer for HashSetConsumer<T>
where
  T: Eq + Hash + Send + std::fmt::Debug + Clone,
{
  async fn consume(&mut self, mut input: Self::InputStream) -> Result<(), Self::Error> {
    while let Some(result) = input.next().await {
      let item = result?;
      match self.on_duplicate {
        DuplicateStrategy::Ignore => {
          self.set.insert(item);
        }
        DuplicateStrategy::Error => {
          if !self.set.insert(item.clone()) {
            return Err(HashSetConsumerError::DuplicateItem(format!("{:?}", item)));
          }
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
    let mut consumer = HashSetConsumer::<i32>::new();
    let input = stream::empty();
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(consumer.is_empty());
    assert_eq!(consumer.len(), 0);
  }

  #[tokio::test]
  async fn test_single_item() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(vec![Ok(42)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert!(!consumer.is_empty());
    assert_eq!(consumer.len(), 1);
    assert!(consumer.into_inner().contains(&42));
  }

  #[tokio::test]
  async fn test_multiple_unique_items() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 3);
    let set = consumer.into_inner();
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
  }

  #[tokio::test]
  async fn test_duplicate_items_error_strategy() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(1)]); // Duplicate 1
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    if let Err(HashSetConsumerError::DuplicateItem(msg)) = result {
      assert!(msg.contains("1"));
    } else {
      panic!("Expected DuplicateItem error");
    }
    assert_eq!(consumer.len(), 2);
    let set = consumer.into_inner();
    assert!(set.contains(&1));
    assert!(set.contains(&2));
  }

  #[tokio::test]
  async fn test_duplicate_items_ignore_strategy() {
    let mut consumer = HashSetConsumer::with_strategy(DuplicateStrategy::Ignore);
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(1), Ok(2), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 3);
    let set = consumer.into_inner();
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
  }

  #[tokio::test]
  async fn test_with_capacity() {
    let mut consumer = HashSetConsumer::<i32>::with_capacity(10);
    let input = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 3);
  }

  #[tokio::test]
  async fn test_string_items() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(vec![
      Ok("hello".to_string()),
      Ok("world".to_string()),
      Ok("!".to_string()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 3);
    let set = consumer.into_inner();
    assert!(set.contains("hello"));
    assert!(set.contains("world"));
    assert!(set.contains("!"));
  }

  #[tokio::test]
  async fn test_complex_type() {
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestStruct {
      id: i32,
      name: String,
    }

    let mut consumer = HashSetConsumer::new();
    let items = vec![
      TestStruct {
        id: 1,
        name: "one".to_string(),
      },
      TestStruct {
        id: 2,
        name: "two".to_string(),
      },
    ];
    let input = stream::iter(items.clone().into_iter().map(Ok));
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 2);
    let set = consumer.into_inner();
    for item in items {
      assert!(set.contains(&item));
    }
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut consumer = HashSetConsumer::<i32>::new();
    let input = stream::iter(vec![
      Ok(1),
      Ok(2),
      Err(HashSetConsumerError::DuplicateItem(
        "test error".to_string(),
      )),
      Ok(3),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    if let Err(HashSetConsumerError::DuplicateItem(msg)) = result {
      assert_eq!(msg, "test error");
    } else {
      panic!("Expected DuplicateItem error");
    }
    assert_eq!(consumer.len(), 2);
  }

  #[tokio::test]
  async fn test_default_implementation() {
    let consumer = HashSetConsumer::<i32>::default();
    assert!(consumer.is_empty());
    assert_eq!(consumer.len(), 0);
    match consumer.on_duplicate {
      DuplicateStrategy::Error => (),
      _ => panic!("Expected Error strategy as default"),
    }
  }

  #[tokio::test]
  async fn test_large_dataset() {
    let mut consumer = HashSetConsumer::with_capacity(1000);
    let items: Vec<i32> = (0..1000).collect();
    let input = stream::iter(items.clone().into_iter().map(Ok));
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
    assert_eq!(consumer.len(), 1000);
    let set = consumer.into_inner();
    for i in 0..1000 {
      assert!(set.contains(&i));
    }
  }

  #[tokio::test]
  async fn test_multiple_consume_calls() {
    let mut consumer = HashSetConsumer::new();

    // First consume
    let input1 = stream::iter(vec![Ok(1), Ok(2), Ok(3)]);
    let result1 = consumer.consume(Box::pin(input1)).await;
    assert!(result1.is_ok());
    assert_eq!(consumer.len(), 3);

    // Second consume - should work with new items
    let input2 = stream::iter(vec![Ok(4), Ok(5), Ok(6)]);
    let result2 = consumer.consume(Box::pin(input2)).await;
    assert!(result2.is_ok());
    assert_eq!(consumer.len(), 6);

    let set = consumer.into_inner();
    for i in 1..=6 {
      assert!(set.contains(&i));
    }
  }
}
