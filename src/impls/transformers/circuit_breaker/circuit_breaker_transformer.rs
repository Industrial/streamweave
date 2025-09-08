use crate::error::ErrorStrategy;
use crate::structs::transformers::circuit_breaker::CircuitBreakerTransformer;
use crate::traits::transformer::TransformerConfig;
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::time::Duration;

impl<T> CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
    Self {
      failure_threshold,
      reset_timeout,
      failure_count: Arc::new(AtomicUsize::new(0)),
      last_failure_time: Arc::new(tokio::sync::RwLock::new(None)),
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

  pub async fn _is_circuit_open(&self) -> bool {
    let failure_count = self.failure_count.load(std::sync::atomic::Ordering::SeqCst);
    if failure_count >= self.failure_threshold {
      let last_failure = self.last_failure_time.read().await;
      if let Some(time) = *last_failure {
        if time.elapsed() >= self.reset_timeout {
          self
            .failure_count
            .store(0, std::sync::atomic::Ordering::SeqCst);
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

  pub async fn _record_failure(&self) {
    self
      .failure_count
      .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut last_failure = self.last_failure_time.write().await;
    *last_failure = Some(tokio::time::Instant::now());
  }
}
