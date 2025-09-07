use crate::error::ErrorStrategy;
use crate::structs::vec_consumer::VecConsumer;
use crate::traits::consumer::ConsumerConfig;

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
