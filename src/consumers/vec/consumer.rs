use super::vec_consumer::VecConsumer;
use crate::consumer::{Consumer, ConsumerConfig};
use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T> Consumer for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);

  async fn consume(&mut self, mut stream: Self::InputStream) -> () {
    let consumer_name = self.config.name.clone();
    println!("📥 [{}] Starting to consume stream", consumer_name);
    let mut count = 0;
    while let Some(value) = stream.next().await {
      count += 1;
      println!(
        "   📦 [{}] Consuming item #{}: {:?}",
        consumer_name, count, value
      );
      self.vec.push(value);
    }
    println!("✅ [{}] Finished consuming {} items", consumer_name, count);
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
  use tokio::runtime::Runtime;

  async fn test_vec_consumer_basic_async(input_data: Vec<i32>) {
    let mut consumer = VecConsumer::new();
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let vec = consumer.into_vec();
    assert_eq!(vec, input_data);
  }

  async fn test_vec_consumer_with_capacity_async(input_data: Vec<i32>, capacity: usize) {
    let mut consumer = VecConsumer::with_capacity(capacity);
    let input = stream::iter(input_data.clone());
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    assert_eq!(consumer.into_vec(), input_data);
  }

  proptest! {
    #[test]
    fn test_vec_consumer_basic(
      input_data in prop::collection::vec(any::<i32>(), 0..30)
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_vec_consumer_basic_async(input_data));
    }

    #[test]
    fn test_vec_consumer_empty_input(_ in prop::num::u8::ANY) {
      let rt = Runtime::new().unwrap();
      rt.block_on(async {
        let mut consumer = VecConsumer::new();
        let input = stream::iter(Vec::<i32>::new());
        let boxed_input = Box::pin(input);

        consumer.consume(boxed_input).await;
        let vec = consumer.into_vec();
        assert!(vec.is_empty());
      });
    }

    #[test]
    fn test_vec_consumer_with_capacity(
      input_data in prop::collection::vec(any::<i32>(), 0..30),
      capacity in 0usize..1000usize
    ) {
      let rt = Runtime::new().unwrap();
      rt.block_on(test_vec_consumer_with_capacity_async(input_data, capacity));
    }

    #[test]
    fn test_error_handling_strategies(
      name in prop::string::string_regex("[a-zA-Z0-9_]+").unwrap()
    ) {
      let consumer = VecConsumer::<i32>::new()
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
