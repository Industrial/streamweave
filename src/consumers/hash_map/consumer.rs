use super::hash_map_consumer::HashMapConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;
use std::hash::Hash;

#[async_trait]
impl<K, V> Consumer for HashMapConsumer<K, V>
where
  K: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
  V: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = ((K, V),);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some((key, value)) = stream.next().await {
      self.map.insert(key, value);
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<(K, V)>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<(K, V)> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<(K, V)> {
    &mut self.config
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
      component_name: self.config.name.clone(),
      component_type: std::any::type_name::<Self>().to_string(),
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
  use crate::error::ErrorStrategy;
  use futures::stream;
  use proptest::prelude::*;
  use proptest::proptest;
  use std::collections::HashMap;
  use tokio::runtime::Runtime;

  async fn test_hash_map_consumer_basic_async(input_data: Vec<(i32, String)>) {
    let mut consumer = HashMapConsumer::new();
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let map = consumer.into_map();

    // Build expected map from input data (later values overwrite earlier ones)
    let mut expected_map = HashMap::new();
    for (key, value) in input_data {
      expected_map.insert(key, value);
    }

    assert_eq!(map.len(), expected_map.len());
    for (key, expected_value) in expected_map {
      assert_eq!(map.get(&key), Some(&expected_value));
    }
  }

  proptest! {
    #[test]
    fn test_hash_map_consumer_basic(
      input_data in prop::collection::vec((any::<i32>(), any::<String>()), 0..30)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_hash_map_consumer_basic_async(input_data));
    }
  }

  async fn test_hash_map_consumer_empty_input_async() {
    let mut consumer = HashMapConsumer::new();
    let input = stream::iter(Vec::<(i32, String)>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert!(consumer.into_map().is_empty());
  }

  proptest! {
    #[test]
    fn test_hash_map_consumer_empty_input(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_hash_map_consumer_empty_input_async());
    }
  }

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = HashMapConsumer::<i32, String>::new()
        .with_error_strategy(ErrorStrategy::<(i32, String)>::Skip)
        .with_name(name.clone());

      prop_assert_eq!(
        &consumer.config().error_strategy,
        &ErrorStrategy::<(i32, String)>::Skip
      );
      prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
    }
  }
}
