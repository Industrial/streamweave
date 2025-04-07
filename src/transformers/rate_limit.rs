use crate::error::{
  ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, PipelineStage, StreamError,
};
use crate::traits::{
  input::Input,
  output::Output,
  transformer::{Transformer, TransformerConfig},
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::time::{Duration, Instant};

pub struct RateLimitTransformer<T>
where
  T: Send + 'static + Clone,
{
  rate_limit: usize,
  time_window: Duration,
  count: Arc<AtomicUsize>,
  window_start: Arc<tokio::sync::RwLock<Instant>>,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> RateLimitTransformer<T>
where
  T: Send + 'static + Clone,
{
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      rate_limit,
      time_window,
      count: Arc::new(AtomicUsize::new(0)),
      window_start: Arc::new(tokio::sync::RwLock::new(Instant::now())),
      config: TransformerConfig::default(),
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

  async fn check_rate_limit(&self) -> Result<(), StreamError<T>> {
    let now = Instant::now();
    let mut window_start = self.window_start.write().await;

    if now.duration_since(*window_start) >= self.time_window {
      self.count.store(0, Ordering::SeqCst);
      *window_start = now;
    }

    if self.count.load(Ordering::SeqCst) >= self.rate_limit {
      Err(StreamError::new(
        Box::new(std::io::Error::new(
          std::io::ErrorKind::Other,
          "Rate limit exceeded",
        )),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer(self.component_info().name),
        },
        self.component_info(),
      ))
    } else {
      self.count.fetch_add(1, Ordering::SeqCst);
      Ok(())
    }
  }
}

impl<T> Input for RateLimitTransformer<T>
where
  T: Send + 'static + Clone,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for RateLimitTransformer<T>
where
  T: Send + 'static + Clone,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for RateLimitTransformer<T>
where
  T: Send + 'static + Clone,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let count = self.count.clone();
    let window_start = self.window_start.clone();
    let rate_limit = self.rate_limit;
    let time_window = self.time_window;

    Box::pin(
      input
        .filter_map(move |item| {
          let count = count.clone();
          let window_start = window_start.clone();
          async move {
            let now = Instant::now();
            let mut window_start = window_start.write().await;

            if now.duration_since(*window_start) >= time_window {
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
        .buffered(1),
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
    match self.config.error_strategy() {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer(self.component_info().name),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .unwrap_or_else(|| "rate_limit_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use futures::stream;
  use tokio::time::sleep;

  #[tokio::test]
  async fn test_rate_limit_basic() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_rate_limit_empty_input() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_rate_limit_actual_rate_limit() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2]);
  }

  #[tokio::test]
  async fn test_rate_limit_reset() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2]);

    sleep(Duration::from_millis(200)).await;

    let input = stream::iter(vec![3, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![3, 4]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
