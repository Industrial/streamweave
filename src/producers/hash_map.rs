use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  output::Output,
  producer::{Producer, ProducerConfig},
};
use futures::{Stream, stream};
use std::collections::HashMap;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;

pub struct HashMapProducer<K, V>
where
  K: Clone + Send + 'static,
  V: Clone + Send + 'static,
{
  data: HashMap<K, V>,
  config: ProducerConfig<(K, V)>,
}

impl<K, V> HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  pub fn new(data: HashMap<K, V>) -> Self {
    Self {
      data,
      config: ProducerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<(K, V)>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl<K, V> Output for HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  type Output = (K, V);
  type OutputStream = Pin<Box<dyn Stream<Item = (K, V)> + Send>>;
}

impl<K, V> Producer for HashMapProducer<K, V>
where
  K: Send + Clone + 'static,
  V: Send + Clone + 'static,
{
  fn produce(&mut self) -> Self::OutputStream {
    let data = self.data.clone();
    Box::pin(stream::iter(data.into_iter()))
  }

  fn set_config_impl(&mut self, config: ProducerConfig<(K, V)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<(K, V)> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<(K, V)> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<(K, V)>) -> ErrorAction {
    match self.config.error_strategy() {
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
      stage: PipelineStage::Producer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "hash_map_producer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
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
    let mut result: Vec<(&str, i32)> = stream.collect().await;

    // Sort for deterministic comparison
    result.sort_by_key(|k| k.0);
    assert_eq!(result, vec![("key1", 1), ("key2", 2), ("key3", 3)]);
  }

  #[tokio::test]
  async fn test_hash_map_producer_empty() {
    let map: HashMap<String, i32> = HashMap::new();
    let mut producer = HashMapProducer::new(map);
    let stream = producer.produce();
    let result: Vec<(String, i32)> = stream.collect().await;
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
    let mut result: Vec<(CustomKey, CustomValue)> = stream.collect().await;

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
    let result1: Vec<(&str, i32)> = stream.collect().await;
    assert_eq!(result1.len(), 1);
    assert_eq!(result1[0], ("test", 42));

    // Second call
    let stream = producer.produce();
    let result2: Vec<(&str, i32)> = stream.collect().await;
    assert_eq!(result2.len(), 1);
    assert_eq!(result2[0], ("test", 42));
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut map = HashMap::new();
    map.insert("test", 42);

    let mut producer = HashMapProducer::new(map)
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
        type_name: "HashMapProducer".to_string(),
      },
      retries: 0,
    };

    assert_eq!(producer.handle_error(&error), ErrorAction::Skip);
  }
}
