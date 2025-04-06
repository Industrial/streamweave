use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  error::Error,
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::collections::HashSet;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;

pub struct HashSetProducer<T> {
  data: HashSet<T>,
  config: ProducerConfig,
}

impl<T> HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
{
  pub fn new(data: HashSet<T>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
    }
  }

  pub fn from_iter<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = T>,
  {
    Self {
      data: iter.into_iter().collect(),
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
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
  type Error = StreamError;
}

impl<T> Output for HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<T, StreamError>> + Send>>;
}

impl<T> Producer for HashSetProducer<T>
where
  T: Clone + Hash + Eq + Send + 'static,
{
  fn produce(&mut self) -> Self::OutputStream {
    let data = self.data.clone();
    Box::pin(stream::iter(data.into_iter().map(Ok)))
  }

  fn config(&self) -> &ProducerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut ProducerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
    }
  }

  fn create_error_context(
    &self,
    item: Option<Arc<dyn std::any::Any + Send + Sync>>,
  ) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "hash_set_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
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

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut set = HashSet::new();
    set.insert(42);

    let mut producer = HashSetProducer::new(set)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      producer.create_error_context(None),
      producer.component_info(),
    );

    assert_eq!(producer.handle_error(error), ErrorStrategy::Skip);
  }
}
