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

pub struct CircuitBreakerTransformer<T: Send + 'static + Clone> {
  failure_threshold: usize,
  reset_timeout: Duration,
  failure_count: Arc<AtomicUsize>,
  last_failure_time: Arc<tokio::sync::RwLock<Option<Instant>>>,
  config: TransformerConfig<T>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T: Send + 'static + Clone> CircuitBreakerTransformer<T> {
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

  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<T>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
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

impl<T: Send + 'static + Clone> Input for CircuitBreakerTransformer<T> {
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T: Send + 'static + Clone> Output for CircuitBreakerTransformer<T> {
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T: Send + 'static + Clone> Transformer for CircuitBreakerTransformer<T> {
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let failure_count = self.failure_count.clone();
    let last_failure_time = self.last_failure_time.clone();
    let failure_threshold = self.failure_threshold;
    let reset_timeout = self.reset_timeout;

    Box::pin(
      input
        .map(move |item| {
          let failure_count = failure_count.clone();
          let last_failure_time = last_failure_time.clone();
          async move {
            failure_count.store(0, Ordering::SeqCst);
            item
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
        .unwrap_or_else(|| "circuit_breaker_transformer".to_string()),
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
  async fn test_circuit_breaker_basic() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(vec![1, 2, 3].into_iter());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_circuit_breaker_empty_input() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100));
    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<i32> = transformer.transform(boxed_input).collect().await;

    assert_eq!(result, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_error_handling_strategies() {
    let mut transformer = CircuitBreakerTransformer::new(3, Duration::from_millis(100))
      .with_error_strategy(ErrorStrategy::<i32>::Skip)
      .with_name("test_transformer".to_string());

    let config = transformer.config();
    assert_eq!(config.error_strategy(), ErrorStrategy::<i32>::Skip);
    assert_eq!(config.name(), Some("test_transformer".to_string()));
  }
}
