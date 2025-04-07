use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::collections::HashSet;
use std::hash::Hash;
use std::pin::Pin;

pub struct HashSetProducer<T>
where
  T: Send + Clone + Hash + Eq + 'static,
{
  data: HashSet<T>,
  config: ProducerConfig<T>,
}

impl<T> HashSetProducer<T>
where
  T: Send + Clone + Hash + Eq + 'static,
{
  pub fn new(data: HashSet<T>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<T> Output for HashSetProducer<T>
where
  T: Send + Clone + Hash + Eq + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Producer for HashSetProducer<T>
where
  T: Send + Clone + Hash + Eq + 'static,
{
  fn produce(&mut self) -> Self::OutputStream {
    let data = self.data.clone();
    Box::pin(stream::iter(data.into_iter()))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
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
    set.insert("item1");
    set.insert("item2");
    set.insert("item3");

    let mut producer = HashSetProducer::new(set.clone());
    let stream = producer.produce();
    let mut result: Vec<&str> = stream.collect().await;

    // Sort for deterministic comparison
    result.sort();
    assert_eq!(result, vec!["item1", "item2", "item3"]);
  }

  #[tokio::test]
  async fn test_hash_set_producer_empty() {
    let set: HashSet<String> = HashSet::new();
    let mut producer = HashSetProducer::new(set);
    let stream = producer.produce();
    let result: Vec<String> = stream.collect().await;
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_hash_set_producer_custom_types() {
    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct CustomItem(String);

    let mut set = HashSet::new();
    set.insert(CustomItem("a".into()));
    set.insert(CustomItem("b".into()));

    let mut producer = HashSetProducer::new(set);
    let stream = producer.produce();
    let mut result: Vec<CustomItem> = stream.collect().await;

    // Sort for deterministic comparison
    result.sort_by_key(|item| item.0.clone());

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "a");
    assert_eq!(result[1].0, "b");
  }

  #[tokio::test]
  async fn test_multiple_produces() {
    let mut set = HashSet::new();
    set.insert("test");

    let mut producer = HashSetProducer::new(set);

    // First call
    let stream = producer.produce();
    let result1: Vec<&str> = stream.collect().await;
    assert_eq!(result1.len(), 1);
    assert_eq!(result1[0], "test");

    // Second call
    let stream = producer.produce();
    let result2: Vec<&str> = stream.collect().await;
    assert_eq!(result2.len(), 1);
    assert_eq!(result2[0], "test");
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut set = HashSet::new();
    set.insert("test");

    let mut producer = HashSetProducer::new(set)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        stage: PipelineStage::Producer,
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "HashSetProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
