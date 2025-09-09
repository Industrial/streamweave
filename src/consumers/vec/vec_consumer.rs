use crate::error::ErrorStrategy;
use crate::consumer::ConsumerConfig;

pub struct VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub vec: Vec<T>,
  pub config: ConsumerConfig<T>,
}

impl<T> Default for VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T> VecConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      vec: Vec::new(),
      config: ConsumerConfig::default(),
    }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      vec: Vec::with_capacity(capacity),
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

  pub fn into_vec(self) -> Vec<T> {
    self.vec
  }
}
