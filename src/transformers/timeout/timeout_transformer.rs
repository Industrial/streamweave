use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use tokio::time::Duration;

#[derive(Clone)]
pub struct TimeoutTransformer<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  pub duration: Duration,
  pub config: TransformerConfig<T>,
  pub _phantom: std::marker::PhantomData<T>,
}

impl<T: std::fmt::Debug + Clone + Send + Sync + 'static> TimeoutTransformer<T> {
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      config: TransformerConfig::<T>::default(),
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}
