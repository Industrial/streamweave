use crate::error::ErrorStrategy;
use crate::structs::transformers::timeout::TimeoutTransformer;
use crate::traits::transformer::TransformerConfig;
use tokio::time::Duration;

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
