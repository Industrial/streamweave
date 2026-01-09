//! Rate limit transformer for StreamWeave

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};

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
  pub window_start: Arc<tokio::sync::RwLock<Instant>>,
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
      window_start: Arc::new(tokio::sync::RwLock::new(Instant::now())),
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
    self.config = self.config.with_error_strategy(strategy);
    self
  }

  /// Sets the name for this transformer.
  ///
  /// # Arguments
  ///
  /// * `name` - The name to assign to this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config = self.config.with_name(name);
    self
  }

  /// Checks if the rate limit has been exceeded.
  ///
  /// This method is protected by a RwLock, so we use relaxed atomic ordering
  /// since the lock already provides the necessary synchronization.
  pub async fn _check_rate_limit(&self) -> Result<(), StreamError<T>> {
    let now = Instant::now();
    let mut window_start = self.window_start.write().await;

    if now.duration_since(*window_start) >= self.time_window {
      self.count.store(0, Ordering::Relaxed);
      *window_start = now;
    }

    if self.count.load(Ordering::Relaxed) >= self.rate_limit {
      Err(StreamError::new(
        Box::new(std::io::Error::other("Rate limit exceeded")),
        ErrorContext {
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

impl<T> Input for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RateLimitTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let rate_limit = self.rate_limit;
    let time_window = self.time_window;
    let count = Arc::clone(&self.count);
    let window_start = Arc::clone(&self.window_start);

    Box::pin(
      input
        .then(move |item| {
          let count = Arc::clone(&count);
          let window_start = Arc::clone(&window_start);
          async move {
            let now = Instant::now();
            let mut window_start = window_start.write().await;

            if now.duration_since(*window_start) > time_window {
              count.store(0, Ordering::SeqCst);
              *window_start = now;
            }

            if count.load(Ordering::SeqCst) >= rate_limit {
              None
            } else {
              count.fetch_add(1, Ordering::SeqCst);
              Some(item)
            }
          }
        })
        .filter_map(futures::future::ready),
    )
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      ErrorStrategy::Custom(handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "rate_limit_transformer".to_string()),
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "rate_limit_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}
