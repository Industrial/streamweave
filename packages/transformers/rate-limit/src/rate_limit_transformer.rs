use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use streamweave_core::{Transformer, TransformerConfig};
use streamweave_error::ErrorStrategy;
use tokio::time::Duration;

/// A transformer that rate limits items in a stream.
///
/// This transformer ensures that only a specified number of items are processed
/// within a given time window, preventing overload and ensuring fair resource usage.
pub struct RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The maximum number of items allowed within the time window.
  pub rate_limit: usize,
  /// The time window for rate limiting.
  pub time_window: Duration,
  /// The current count of items processed in the current window.
  pub count: Arc<AtomicUsize>,
  /// The start time of the current rate limiting window.
  pub window_start: Arc<tokio::sync::RwLock<tokio::time::Instant>>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `RateLimitTransformer` with the given rate limit and time window.
  ///
  /// # Arguments
  ///
  /// * `rate_limit` - The maximum number of items allowed within the time window.
  /// * `time_window` - The time window for rate limiting.
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      rate_limit,
      time_window,
      count: Arc::new(AtomicUsize::new(0)),
      window_start: Arc::new(tokio::sync::RwLock::new(tokio::time::Instant::now())),
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

  /// Checks if the rate limit has been exceeded.
  ///
  /// This method is protected by a RwLock, so we use relaxed atomic ordering
  /// since the lock already provides the necessary synchronization.
  pub async fn _check_rate_limit(&self) -> Result<(), streamweave_error::StreamError<T>> {
    let now = tokio::time::Instant::now();
    let mut window_start = self.window_start.write().await;

    if now.duration_since(*window_start) >= self.time_window {
      self.count.store(0, Ordering::Relaxed);
      *window_start = now;
    }

    if self.count.load(Ordering::Relaxed) >= self.rate_limit {
      Err(streamweave_error::StreamError::new(
        Box::new(std::io::Error::other("Rate limit exceeded")),
        streamweave_error::ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          component_name: self.component_info().name,
          component_type: std::any::type_name::<Self>().to_string(),
        },
        self.component_info(),
      ))
    } else {
      self.count.fetch_add(1, Ordering::Relaxed);
      Ok(())
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use streamweave_error::{ErrorStrategy, StreamError};

  #[test]
  fn test_rate_limit_transformer_new() {
    let transformer = RateLimitTransformer::<i32>::new(10, Duration::from_secs(1));
    assert_eq!(transformer.rate_limit, 10);
    assert_eq!(transformer.time_window, Duration::from_secs(1));
    assert_eq!(transformer.count.load(Ordering::Relaxed), 0);
  }

  #[test]
  fn test_rate_limit_transformer_with_error_strategy() {
    let transformer = RateLimitTransformer::<i32>::new(5, Duration::from_secs(1))
      .with_error_strategy(ErrorStrategy::<i32>::Skip);
    assert!(matches!(
      transformer.config.error_strategy,
      ErrorStrategy::Skip
    ));
  }

  #[test]
  fn test_rate_limit_transformer_with_name() {
    let transformer = RateLimitTransformer::<i32>::new(5, Duration::from_secs(1))
      .with_name("test_rate_limit".to_string());
    assert_eq!(transformer.config.name, Some("test_rate_limit".to_string()));
  }

  #[tokio::test]
  async fn test_check_rate_limit_within_limit() {
    let transformer = RateLimitTransformer::<i32>::new(5, Duration::from_secs(1));
    let result = transformer._check_rate_limit().await;
    assert!(result.is_ok());
    assert_eq!(transformer.count.load(Ordering::Relaxed), 1);
  }

  #[tokio::test]
  async fn test_check_rate_limit_exceeded() {
    let transformer = RateLimitTransformer::<i32>::new(2, Duration::from_secs(1));

    // First two should succeed
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());

    // Third should fail
    let result = transformer._check_rate_limit().await;
    assert!(result.is_err());
    let _: StreamError<i32> = result.unwrap_err();
  }

  #[tokio::test]
  async fn test_check_rate_limit_window_reset() {
    let transformer = RateLimitTransformer::<i32>::new(2, Duration::from_millis(100));

    // Use up the limit
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_err());

    // Wait for window to reset
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should work again after window reset
    let result = transformer._check_rate_limit().await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_check_rate_limit_multiple_windows() {
    let transformer = RateLimitTransformer::<i32>::new(3, Duration::from_millis(50));

    // First window
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_err());

    // Wait for window reset
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second window
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_ok());
    assert!(transformer._check_rate_limit().await.is_err());
  }

  #[tokio::test]
  async fn test_check_rate_limit_error_context() {
    let transformer = RateLimitTransformer::<i32>::new(1, Duration::from_secs(1));

    // Use the limit
    assert!(transformer._check_rate_limit().await.is_ok());

    // This should fail and create an error
    let result = transformer._check_rate_limit().await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(!error.component.name.is_empty());
    assert!(!error.component.type_name.is_empty());
  }
}
