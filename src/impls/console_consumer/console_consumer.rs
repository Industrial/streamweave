use crate::error::ErrorStrategy;
use crate::structs::console_consumer::ConsoleConsumer;
use crate::traits::consumer::ConsumerConfig;

impl<T> ConsoleConsumer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + std::fmt::Display + 'static,
{
  pub fn new() -> Self {
    Self {
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
}
