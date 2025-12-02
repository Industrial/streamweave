use super::string_consumer::StringConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl Consumer for StringConsumer {
  type InputPorts = (String,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      self.buffer.push_str(&value);
    }
  }

  fn set_config_impl(&mut self, config: ConsumerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ConsumerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ConsumerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
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
  use tokio::runtime::Runtime;

  async fn test_string_consumer_basic_async(input_data: Vec<String>) {
    let mut consumer = StringConsumer::new();
    let expected = input_data.concat();
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(consumer.into_string(), expected);
  }

  async fn test_string_consumer_with_capacity_async(input_data: Vec<String>, capacity: usize) {
    let mut consumer = StringConsumer::with_capacity(capacity);
    let expected = input_data.concat();
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(consumer.into_string(), expected);
  }

  proptest! {
    #[test]
    fn test_string_consumer_basic(
      input_data in prop::collection::vec(any::<String>(), 0..30)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_string_consumer_basic_async(input_data));
    }

    #[test]
    fn test_string_consumer_empty_input(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
      rt.block_on(async {
        let mut consumer = StringConsumer::new();
        let input = stream::iter(Vec::<String>::new());
        let boxed_input = Box::pin(input);

        consumer.consume(boxed_input).await;
        assert!(consumer.into_string().is_empty());
      });
    }

    #[test]
    fn test_string_consumer_with_capacity(
      input_data in prop::collection::vec(any::<String>(), 0..30),
      capacity in 0usize..1000usize
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_string_consumer_with_capacity_async(input_data, capacity));
    }

    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = StringConsumer::new()
        .with_error_strategy(ErrorStrategy::<String>::Skip)
        .with_name(name.clone());

      prop_assert_eq!(
        &consumer.config().error_strategy,
        &ErrorStrategy::<String>::Skip
      );
      prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
    }
  }
}
