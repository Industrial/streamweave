pub mod consumer;
pub mod input;

use crate::error::ErrorStrategy;
use crate::traits::consumer::ConsumerConfig;

pub struct ArrayConsumer<T, const N: usize>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub array: [Option<T>; N],
  pub index: usize,
  pub config: ConsumerConfig<T>,
}

impl<T, const N: usize> ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      array: std::array::from_fn(|_| None),
      index: 0,
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = name;
    self
  }

  pub fn into_array(self) -> [Option<T>; N] {
    self.array
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::consumer::Consumer;
  use futures::stream;

  #[tokio::test]
  async fn test_array_consumer_basic() {
    let mut consumer = ArrayConsumer::<i32, 3>::new();
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    assert_eq!(array[0], Some(1));
    assert_eq!(array[1], Some(2));
    assert_eq!(array[2], Some(3));
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

  #[tokio::test]
  async fn test_array_consumer_capacity_exceeded() {
    let mut consumer = ArrayConsumer::<i32, 2>::new();
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    consumer.consume(boxed_input).await;
    let array = consumer.into_array();
    assert_eq!(array[0], Some(1));
    assert_eq!(array[1], Some(2));
  }
}

// Re-export the trait implementations
// Note: The trait implementations are automatically available through the module structure
