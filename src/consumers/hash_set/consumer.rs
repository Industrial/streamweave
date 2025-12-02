use super::hash_set_consumer::HashSetConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;
use std::hash::Hash;

#[async_trait]
impl<T> Consumer for HashSetConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + Hash + Eq + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      self.set.insert(value);
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match self.config.error_strategy {
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
  use std::collections::HashSet;
  use tokio::runtime::Runtime;

  async fn test_hash_set_consumer_basic_async(input_data: Vec<i32>) {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let set = consumer.into_set();

    // Build expected set (HashSet automatically handles duplicates)
    let expected_set: HashSet<i32> = input_data.into_iter().collect();

    assert_eq!(set.len(), expected_set.len());
    for item in expected_set {
      assert!(set.contains(&item));
    }
  }

  proptest! {
    #[test]
    fn test_hash_set_consumer_basic(input_data in prop::collection::vec(any::<i32>(), 0..30)) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_hash_set_consumer_basic_async(input_data));
    }
  }

  async fn test_hash_set_consumer_empty_input_async() {
    let mut consumer = HashSetConsumer::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert!(consumer.into_set().is_empty());
  }

  proptest! {
    #[test]
    fn test_hash_set_consumer_empty_input(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_hash_set_consumer_empty_input_async());
    }
  }

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = HashSetConsumer::<i32>::new()
        .with_error_strategy(ErrorStrategy::<i32>::Skip)
        .with_name(name.clone());

      prop_assert_eq!(
        &consumer.config().error_strategy,
        &ErrorStrategy::<i32>::Skip
      );
      prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
    }
  }
}
