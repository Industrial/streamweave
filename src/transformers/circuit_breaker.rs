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

pub struct CircuitBreakerTransformer<T> {
  failure_threshold: usize,
  reset_timeout: Duration,
  failure_count: Arc<AtomicUsize>,
  last_failure_time: Arc<tokio::sync::RwLock<Option<Instant>>>,
  config: TransformerConfig,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> CircuitBreakerTransformer<T>
where
  T: Send + 'static,
{
  pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
    Self {
      failure_threshold,
      reset_timeout,
      failure_count: Arc::new(AtomicUsize::new(0)),
      last_failure_time: Arc::new(tokio::sync::RwLock::new(None)),
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

  async fn is_circuit_open(&self) -> bool {
    let failure_count = self.failure_count.load(Ordering::SeqCst);
    if failure_count >= self.failure_threshold {
      let last_failure = self.last_failure_time.read().await;
      if let Some(time) = *last_failure {
        if time.elapsed() >= self.reset_timeout {
          self.failure_count.store(0, Ordering::SeqCst);
          false
        } else {
          true
        }
      } else {
        true
      }
    } else {
      false
    }
  }

  async fn record_failure(&self) {
    self.failure_count.fetch_add(1, Ordering::SeqCst);
    let mut last_failure = self.last_failure_time.write().await;
    *last_failure = Some(Instant::now());
  }
}

impl<T> crate::traits::error::Error for CircuitBreakerTransformer<T>
where
  T: Send + 'static,
{
  type Error = StreamError;
}

impl<T> Input for CircuitBreakerTransformer<T>
where
  T: Send + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = Result<Self::Input, StreamError>> + Send>>;
}

impl<T> Output for CircuitBreakerTransformer<T>
where
  T: Send + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, StreamError>> + Send>>;
}

#[async_trait]
impl<T> Transformer for CircuitBreakerTransformer<T>
where
  T: Send + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let failure_count = self.failure_count.clone();
    let last_failure_time = self.last_failure_time.clone();
    let failure_threshold = self.failure_threshold;
    let reset_timeout = self.reset_timeout;

    Box::pin(
      input
        .map(move |result| {
          let failure_count = failure_count.clone();
          let last_failure_time = last_failure_time.clone();
          async move {
            match result {
              Ok(item) => {
                failure_count.store(0, Ordering::SeqCst);
                Ok(item)
              }
              Err(e) => {
                failure_count.fetch_add(1, Ordering::SeqCst);
                let mut last_failure = last_failure_time.write().await;
                *last_failure = Some(Instant::now());

                if failure_count.load(Ordering::SeqCst) >= failure_threshold {
                  if let Some(time) = *last_failure {
                    if time.elapsed() < reset_timeout {
                      return Err(StreamError::new(
                        Box::new(std::io::Error::new(
                          std::io::ErrorKind::Other,
                          "Circuit breaker is open",
                        )),
                        ErrorContext {
                          timestamp: chrono::Utc::now(),
                          item: None,
                          stage: PipelineStage::Transformer,
                        },
                        ComponentInfo {
                          name: "circuit_breaker_transformer".to_string(),
                          type_name: std::any::type_name::<Self>().to_string(),
                        },
                      ));
                    }
                  }
                }
                Err(e)
              }
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
        .unwrap_or_else(|| "circuit_breaker_transformer".to_string()),
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
  async fn test_circuit_breaker_basic() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
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
  async fn test_circuit_breaker_empty_input() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
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
  async fn test_circuit_breaker_with_error() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
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
  async fn test_circuit_breaker_actual_circuit_breaker() {
    let mut transformer = CircuitBreakerTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(|x| {
      if x == 2 {
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
        ))
      } else {
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());
    if let Err(e) = result {
      assert_eq!(e.source.to_string(), "Circuit breaker is open");
    }
  }

  #[tokio::test]
  async fn test_circuit_breaker_reset() {
    let mut transformer = CircuitBreakerTransformer::new(2, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter().map(|x| {
      if x == 2 {
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
        ))
      } else {
        Ok(x)
      }
    }));
    let boxed_input = Box::pin(input);

    let result: Result<Vec<i32>, _> = transformer.transform(boxed_input).try_collect().await;

    assert!(result.is_err());

    sleep(Duration::from_millis(200)).await;

    let input = stream::iter(vec![4, 5, 6].into_iter().map(Ok));
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer
      .transform(boxed_input)
      .try_collect()
      .await
      .unwrap();

    assert_eq!(result, vec![4, 5, 6]);
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100))
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
