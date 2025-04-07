use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  consumer::{Consumer, ConsumerConfig},
  input::Input,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;

pub struct HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  map: HashMap<K, V>,
  config: ConsumerConfig<(K, V)>,
}

impl<K, V> HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      map: HashMap::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(K, V)>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  pub fn into_map(self) -> HashMap<K, V> {
    self.map
  }
}

impl<K, V> Input for HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = (K, V);
  type InputStream = Pin<Box<dyn Stream<Item = (K, V)> + Send>>;
}

#[async_trait]
impl<K, V> Consumer for HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some((key, value)) = stream.next().await {
      self.map.insert(key, value);
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<(K, V)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> ConsumerConfig<(K, V)> {
    self.config.clone()
  }

  fn handle_error(&self, error: &StreamError<(K, V)>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<(K, V)>) -> ErrorContext<(K, V)> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Consumer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self.config.name.clone(),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

  #[tokio::test]
  async fn test_hash_map_consumer_basic() {
    let mut consumer = HashMapConsumer::new();
    let input = stream::iter(vec![
      (1, "one"),
      (2, "two"),
      (3, "three"),
      (1, "one_updated"),
    ]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let map = consumer.into_map();
    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&1), Some(&"one_updated"));
    assert_eq!(map.get(&2), Some(&"two"));
    assert_eq!(map.get(&3), Some(&"three"));
  }

  #[tokio::test]
  async fn test_hash_map_consumer_empty_input() {
    let mut consumer = HashMapConsumer::new();
    let input = stream::iter(Vec::<(i32, &str)>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert!(consumer.into_map().is_empty());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut consumer = HashMapConsumer::new()
      .with_error_strategy(ErrorStrategy::<(i32, &str)>::Skip)
      .with_name("test_consumer".to_string());

    let config = consumer.get_config();
    assert_eq!(config.error_strategy, ErrorStrategy::<(i32, &str)>::Skip);
    assert_eq!(config.name, "test_consumer");
  }
}
