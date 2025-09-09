use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;
use tokio::time::Duration;

pub struct RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub max_retries: usize,
  pub backoff: Duration,
  pub config: TransformerConfig<T>,
  pub _phantom: PhantomData<T>,
}

impl<T> Clone for RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  fn clone(&self) -> Self {
    Self {
      max_retries: self.max_retries,
      backoff: self.backoff,
      config: self.config.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<T> RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(max_retries: usize, backoff: Duration) -> Self {
    Self {
      max_retries,
      backoff,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
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

  pub fn max_retries(&self) -> usize {
    self.max_retries
  }

  pub fn backoff(&self) -> Duration {
    self.backoff
  }
}
