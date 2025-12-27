use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use streamweave_core::TransformerConfig;
use streamweave_error::ErrorStrategy;
use tokio::time::Duration;

/// A transformer that implements the circuit breaker pattern.
///
/// This transformer monitors failures and opens the circuit (stops processing)
/// when the failure threshold is exceeded, automatically resetting after a timeout.
#[derive(Clone)]
pub struct CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// The number of failures before the circuit opens.
  pub failure_threshold: usize,
  /// The duration to wait before attempting to reset the circuit.
  pub reset_timeout: Duration,
  /// The current count of failures.
  pub failure_count: Arc<AtomicUsize>,
  /// The timestamp of the last failure.
  pub last_failure_time: Arc<tokio::sync::RwLock<Option<tokio::time::Instant>>>,
  /// Configuration for the transformer, including error handling strategy.
  pub config: TransformerConfig<T>,
  /// Phantom data to track the type parameter.
  pub _phantom: PhantomData<T>,
}

impl<T> CircuitBreakerTransformer<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `CircuitBreakerTransformer` with the given failure threshold and reset timeout.
  ///
  /// # Arguments
  ///
  /// * `failure_threshold` - The number of failures before the circuit opens.
  /// * `reset_timeout` - The duration to wait before attempting to reset the circuit.
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

  /// Checks if the circuit breaker is open (tripped).
  ///
  /// Uses Acquire ordering on load and Release on store to ensure proper
  /// visibility across threads while avoiding the overhead of SeqCst.
  pub async fn _is_circuit_open(&self) -> bool {
    let failure_count = self.failure_count.load(Ordering::Acquire);
    if failure_count >= self.failure_threshold {
      let last_failure = self.last_failure_time.read().await;
      if let Some(time) = *last_failure {
        if time.elapsed() >= self.reset_timeout {
          self.failure_count.store(0, Ordering::Release);
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

  /// Records a failure, incrementing the failure count.
  ///
  /// Uses AcqRel ordering to ensure the increment is visible to other threads
  /// and we see the latest count value.
  pub async fn _record_failure(&self) {
    self.failure_count.fetch_add(1, Ordering::AcqRel);
    let mut last_failure = self.last_failure_time.write().await;
    *last_failure = Some(tokio::time::Instant::now());
  }
}
