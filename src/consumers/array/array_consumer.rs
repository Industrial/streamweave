use crate::error::ErrorStrategy;
use crate::consumer::ConsumerConfig;

pub struct ArrayConsumer<T, const N: usize>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub array: [Option<T>; N],
  pub index: usize,
  pub config: ConsumerConfig<T>,
}

impl<T, const N: usize> Default for ArrayConsumer<T, N>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
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
