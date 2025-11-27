use crate::error::ErrorStrategy;
use crate::transformer::{Transformer, TransformerConfig};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
  pub async fn _check_rate_limit(&self) -> Result<(), crate::error::StreamError<T>> {
    let now = tokio::time::Instant::now();
    let mut window_start = self.window_start.write().await;

    if now.duration_since(*window_start) >= self.time_window {
      self.count.store(0, Ordering::Relaxed);
      *window_start = now;
    }

    if self.count.load(Ordering::Relaxed) >= self.rate_limit {
      Err(crate::error::StreamError::new(
        Box::new(std::io::Error::other("Rate limit exceeded")),
        crate::error::ErrorContext {
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
