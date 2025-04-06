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

pub struct RateLimitTransformer<T> {
  rate_limit: usize,
  time_window: Duration,
  count: Arc<AtomicUsize>,
  window_start: Arc<tokio::sync::RwLock<Instant>>,
  config: TransformerConfig,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> RateLimitTransformer<T>
where
  T: Send + 'static,
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

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
    self.config_mut().set_error_strategy(strategy);
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config_mut().set_name(name);
    self
  }

  async fn check_rate_limit(&self) -> Result<(), StreamError> {
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
          stage: PipelineStage::Transformer,
        },
        ComponentInfo {
          name: "rate_limit_transformer".to_string(),
          type_name: std::any::type_name::<Self>().to_string(),
        },
      ))
    } else {
      self.count.fetch_add(1, Ordering::SeqCst);
      Ok(())
    }
  }
}

impl<T> crate::traits::error::Error for RateLimitTransformer<T>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T> Input for RateLimitTransformer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T> Output for RateLimitTransformer<T>
where
  T: Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for RateLimitTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let count = self.count.clone();
    let window_start = self.window_start.clone();
    let rate_limit = self.rate_limit;
    let time_window = self.time_window;

    Box::pin(
      input
        .map(move |result| {
          let count = count.clone();
          let window_start = window_start.clone();
          async move {
            match result {
              Ok(item) => {
                let now = Instant::now();
                let mut window_start = window_start.write().await;

                if now.duration_since(*window_start) >= time_window {
                  count.store(0, Ordering::SeqCst);
                  *window_start = now;
                }

                if count.load(Ordering::SeqCst) >= rate_limit {
                  Err(StreamError::new(
                    Box::new(std::io::Error::new(
                      std::io::ErrorKind::Other,
                      "Rate limit exceeded",
                    )),
                    ErrorContext {
                      timestamp: chrono::Utc::now(),
                      item: None,
                      stage: PipelineStage::Transformer,
                    },
                    ComponentInfo {
                      name: "rate_limit_transformer".to_string(),
                      type_name: std::any::type_name::<Self>().to_string(),
                    },
                  ))
                } else {
                  count.fetch_add(1, Ordering::SeqCst);
                  Ok(item)
                }
              }
              Err(e) => Err(e),
            }
          }
        })
        .buffered(1),
    )
  }

  fn config(&self) -> &TransformerConfig {
    &self.config
  }

  fn config_mut(&mut self) -> &mut TransformerConfig {
    &mut self.config
  }

  fn handle_error(&self, error: StreamError) -> ErrorStrategy {
    match self.config().error_strategy() {
      ErrorStrategy::Stop => ErrorStrategy::Stop,
      ErrorStrategy::Skip => ErrorStrategy::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorStrategy::Retry(n),
      _ => ErrorStrategy::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Box<dyn std::any::Any + Send>>) -> ErrorContext {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      stage: PipelineStage::Transformer,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config()
        .name()
        .unwrap_or_else(|| "rate_limit_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::TryStreamExt;
  use futures::stream;
  use tokio::time::sleep;

  #[tokio::test]
  async fn test_rate_limit_basic() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_rate_limit_empty_input() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(Vec::<Result<i32, StreamError>>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_rate_limit_with_error() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![
      Ok(1),
      Err(StreamError::new(
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
        ErrorContext {
          timestamp: chrono::Utc::now(),
          item: None,
          stage: PipelineStage::Transformer,
        },
        ComponentInfo {
          name: "test".to_string(),
          type_name: "test".to_string(),
        },
      )),
      Ok(3),
    ]);
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_rate_limit_actual_rate_limit() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
    if let Err(e) = result {
      assert_eq!(e.source.to_string(), "Rate limit exceeded");
    }
  }

  #[tokio::test]
  async fn test_rate_limit_reset() {
    let mut transformer = RateLimitTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![1, 2]);

    sleep(Duration::from_millis(200)).await;

    let input = stream::iter(vec![3, 4].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![3, 4]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = RateLimitTransformer::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));

    let error = StreamError::new(
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error")),
      transformer.create_error_context(None),
      transformer.component_info(),
    );

    assert_eq!(transformer.handle_error(error), ErrorStrategy::Skip);
  }
}
