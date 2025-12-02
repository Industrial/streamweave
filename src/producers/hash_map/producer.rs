use super::hash_map_producer::HashMapProducer;
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::producer::{Producer, ProducerConfig};
use futures::stream;
use std::hash::Hash;

impl<K, V> Producer for HashMapProducer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type OutputPorts = ((K, V),);

  fn produce(&mut self) -> Self::OutputStream {
    let data = self.data.clone();
    Box::pin(stream::iter(data))
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
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "hash_map_producer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
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
  use std::collections::HashMap;

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

    let producer = HashMapProducer::new(map)
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_producer".to_string());

    let config = producer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_producer".to_string()));

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "HashMapProducer".to_string(),
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
