use crate::error::ErrorStrategy;
use crate::structs::array_consumer::ArrayConsumer;
use crate::traits::consumer::ConsumerConfig;

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
