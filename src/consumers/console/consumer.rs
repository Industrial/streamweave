use super::console_consumer::ConsoleConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Consumer for ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      println!("{}", value);
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

  async fn test_console_consumer_integers_async(input: Vec<i32>) {
    let mut consumer = ConsoleConsumer::<i32>::new();
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_console_consumer_integers(
      input in prop::collection::vec(-1000..1000i32, 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_integers_async(input));
    }
  }

  async fn test_console_consumer_strings_async(input: Vec<String>) {
    let mut consumer = ConsoleConsumer::<String>::new();
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_console_consumer_strings(
      input in prop::collection::vec(prop::string::string_regex("[a-zA-Z0-9 ]+").unwrap(), 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_strings_async(input));
    }
  }

  async fn test_console_consumer_floats_async(input: Vec<f64>) {
    let mut consumer = ConsoleConsumer::<f64>::new();
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_console_consumer_floats(
      input in prop::collection::vec(-1000.0..1000.0f64, 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_floats_async(input));
    }
  }

  async fn test_console_consumer_custom_type_async(values: Vec<i32>) {
    #[derive(Debug, Clone)]
    struct CustomType(i32);

    impl std::fmt::Display for CustomType {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Custom({})", self.0)
      }
    }

    unsafe impl Send for CustomType {}
    unsafe impl Sync for CustomType {}

    let mut consumer = ConsoleConsumer::<CustomType>::new();
    let input: Vec<CustomType> = values.iter().map(|v| CustomType(*v)).collect();
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
  }

  proptest! {
    #[test]
    fn test_console_consumer_custom_type(
      values in prop::collection::vec(-1000..1000i32, 0..100)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_custom_type_async(values));
    }
  }

  async fn test_console_consumer_reuse_async(input1: Vec<i32>, input2: Vec<i32>) {
    let mut consumer = ConsoleConsumer::<i32>::new();

    // First consumption
    let input1_clone = input1.clone();
    let input_stream1 = stream::iter(input1_clone);
    let boxed_input1 = Box::pin(input_stream1);
    consumer.consume(boxed_input1).await;

    // Second consumption - should work fine
    let input2_clone = input2.clone();
    let input_stream2 = stream::iter(input2_clone);
    let boxed_input2 = Box::pin(input_stream2);
    consumer.consume(boxed_input2).await;
  }

  proptest! {
    #[test]
    fn test_console_consumer_reuse(
      input1 in prop::collection::vec(-1000..1000i32, 0..50),
      input2 in prop::collection::vec(-1000..1000i32, 0..50)
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_console_consumer_reuse_async(input1, input2));
    }
  }

  async fn test_error_handling_strategies_async(name: String) {
    let consumer = ConsoleConsumer::<i32>::new()
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name(name.clone());

    assert_eq!(consumer.config().error_strategy, ErrorStrategy::<i32>::Skip);
    assert_eq!(consumer.config().name, name);
  }

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_error_handling_strategies_async(name));
    }
  }
}
