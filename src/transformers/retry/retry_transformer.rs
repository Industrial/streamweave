use crate::error::ErrorStrategy;
use crate::transformer::TransformerConfig;
use std::marker::PhantomData;
use tokio::time::Duration;

/// A transformer that retries failed operations with configurable backoff.
///
/// This transformer automatically retries failed operations up to a maximum number
/// of times, with a configurable backoff delay between retries.
pub struct RetryTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum number of retry attempts.
  pub max_retries: usize,
  /// The backoff duration between retry attempts.
  pub backoff: Duration,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
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
  /// Creates a new `RetryTransformer` with the given maximum retries and backoff duration.
  ///
  /// # Arguments
  ///
  /// * `max_retries` - The maximum number of retry attempts.
  /// * `backoff` - The backoff duration between retry attempts.
  pub fn new(max_retries: usize, backoff: Duration) -> Self {
    Self {
      max_retries,
      backoff,
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }

  /// Sets the error handling strategy for this transformer.
  ///
  /// # Arguments
  ///
  /// * `strategy` - The error handling strategy to use.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }

  /// Returns the maximum number of retry attempts.
  pub fn max_retries(&self) -> usize {
    self.max_retries
  }

  /// Returns the backoff duration between retry attempts.
  pub fn backoff(&self) -> Duration {
    self.backoff
  }
}
