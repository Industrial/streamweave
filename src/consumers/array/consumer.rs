use super::array_consumer::ArrayConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T, const N: usize> Consumer for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    while let Some(value) = stream.next().await {
      if self.index < N {
        self.array[self.index] = Some(value);
        self.index += 1;
      }
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
  use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
  use futures::stream;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = ArrayConsumer::<i32, 3>::new()
        .with_error_strategy(ErrorStrategy::<i32>::Skip)
        .with_name(name.clone());

      prop_assert!(matches!(consumer.config().error_strategy, ErrorStrategy::<i32>::Skip));
      prop_assert_eq!(consumer.config().name.as_str(), name.as_str());
    }

    #[test]
    fn test_error_handling_during_consumption(
      error_msg in prop::string::string_regex(".+").unwrap(),
      item in -1000..1000i32
    ) {
      let consumer = ArrayConsumer::<i32, 3>::new()
        .with_error_strategy(ErrorStrategy::<i32>::Skip)
        .with_name("test_consumer".to_string());

      // Test that Skip strategy allows consumption to continue
      let action = consumer.handle_error(&StreamError {
        source: Box::new(std::io::Error::other(error_msg.clone())),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: Some(item),
          component_name: "test".to_string(),
          component_type: "test".to_string(),
        },
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
        retries: 0,
      });
      prop_assert_eq!(action, ErrorAction::Skip);

      // Test that Stop strategy halts consumption
      let consumer = consumer.with_error_strategy(ErrorStrategy::<i32>::Stop);
      let action = consumer.handle_error(&StreamError {
        source: Box::new(std::io::Error::other(error_msg)),
        context: ErrorContext {
          timestamp: chrono::Utc::now(),
          item: Some(item),
          component_name: "test".to_string(),
          component_type: "test".to_string(),
        },
        component: ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
        retries: 0,
      });
      prop_assert_eq!(action, ErrorAction::Stop);
    }
  }

  proptest! {
    #[test]
    fn test_component_info(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = ArrayConsumer::<i32, 3>::new().with_name(name.clone());

      let info = consumer.component_info();
      prop_assert_eq!(info.name.as_str(), name.as_str());
      prop_assert_eq!(
        info.type_name,
        "streamweave::consumers::array::array_consumer::ArrayConsumer<i32, 3>"
      );
    }

    #[test]
    fn test_error_context_creation(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap(),
      item in -1000..1000i32
    ) {
      let consumer = ArrayConsumer::<i32, 3>::new().with_name(name.clone());

      let context = consumer.create_error_context(Some(item));
      prop_assert_eq!(context.component_name.as_str(), name.as_str());
      prop_assert_eq!(
        context.component_type,
        "streamweave::consumers::array::array_consumer::ArrayConsumer<i32, 3>"
      );
      prop_assert_eq!(context.item, Some(item));
    }
  }

  async fn test_array_consumer_basic_async(input: Vec<i32>) {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    for (i, val) in input.iter().take(3).enumerate() {
      assert_eq!(array[i], Some(*val));
    }
  }

  #[test]
  fn test_array_consumer_basic() {
    proptest::proptest!(|(input in prop::collection::vec(-1000..1000i32, 1..10))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_array_consumer_basic_async(input));
    });
  }

  #[tokio::test]
  async fn test_array_consumer_empty_input() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    assert_eq!(array[0], None);
    assert_eq!(array[1], None);
    assert_eq!(array[2], None);
  }

  async fn test_array_consumer_capacity_exceeded_async(input: Vec<i32>, _capacity: usize) {
    let mut consumer = ArrayConsumer::<i32, 2>::new();
    let input_clone = input.clone();
    let input_stream = stream::iter(input_clone);
    let boxed_input = Box::pin(input_stream);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    // Should only store up to capacity (2)
    assert_eq!(array[0], input.first().copied());
    assert_eq!(array[1], input.get(1).copied());
  }

  #[test]
  fn test_array_consumer_capacity_exceeded() {
    proptest::proptest!(|(input in prop::collection::vec(-1000..1000i32, 3..20))| {
      let rt = tokio::runtime::Runtime::new().unwrap();
      rt.block_on(test_array_consumer_capacity_exceeded_async(input, 2));
    });
  }
}
