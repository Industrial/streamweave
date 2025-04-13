use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

pub struct CircuitBreakerOperator<T>
where
  T: Send + Sync + 'static,
{
  failure_threshold: usize,
  reset_timeout: Duration,
  failure_count: Arc<AtomicUsize>,
  last_failure_time: Arc<tokio::sync::RwLock<Option<Instant>>>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> CircuitBreakerOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(failure_threshold: usize, reset_timeout: Duration) -> Self {
    Self {
      failure_threshold,
      reset_timeout,
      failure_count: Arc::new(AtomicUsize::new(0)),
      last_failure_time: Arc::new(tokio::sync::RwLock::new(None)),
      _phantom: std::marker::PhantomData,
    }
  }

  #[allow(dead_code)]
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

  #[allow(dead_code)]
  async fn record_failure(&self) {
    self.failure_count.fetch_add(1, Ordering::SeqCst);
    let mut last_failure = self.last_failure_time.write().await;
    *last_failure = Some(Instant::now());
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for CircuitBreakerOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let failure_count = self.failure_count.clone();
    let last_failure_time = self.last_failure_time.clone();
    let failure_threshold = self.failure_threshold;
    let reset_timeout = self.reset_timeout;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let stream_clone = stream_clone;
        let failure_count = failure_count.clone();
        let last_failure_time = last_failure_time.clone();

        while let Ok(Some(item)) = stream_clone.next().await {
          if failure_count.load(Ordering::SeqCst) >= failure_threshold {
            let last_failure = last_failure_time.read().await;
            if let Some(time) = *last_failure {
              if time.elapsed() >= reset_timeout {
                failure_count.store(0, Ordering::SeqCst);
                new_stream_clone.push(item).await.unwrap();
              }
            }
          } else {
            new_stream_clone.push(item).await.unwrap();
          }
        }
        new_stream_clone.close().await.unwrap();
      });

      Ok(new_stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::time::sleep;

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_circuit_breaker_basic() {
    let operator = CircuitBreakerOperator::new(3, Duration::from_millis(100));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_circuit_breaker_empty_input() {
    let operator = CircuitBreakerOperator::new(3, Duration::from_millis(100));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_circuit_breaker_failure_threshold() {
    let operator = CircuitBreakerOperator::new(2, Duration::from_millis(100));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    let producer = tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
      if value == 2 {
        operator.failure_count.store(2, Ordering::SeqCst);
        let mut last_failure = operator.last_failure_time.write().await;
        *last_failure = Some(Instant::now());
      }
    }

    producer.await.unwrap();

    assert_eq!(results, vec![1, 2]);
  }

  #[tokio::test]
  async fn test_circuit_breaker_reset() {
    let operator = CircuitBreakerOperator::new(2, Duration::from_millis(100));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      sleep(Duration::from_millis(150)).await;
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
      if value == 2 {
        operator.failure_count.store(2, Ordering::SeqCst);
        let mut last_failure = operator.last_failure_time.write().await;
        *last_failure = Some(Instant::now());
      }
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_circuit_breaker_concurrent() {
    let operator = CircuitBreakerOperator::new(2, Duration::from_millis(100));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let new_stream = operator.transform(stream).await.unwrap();

    // Create multiple consumer tasks before producing any values
    let num_consumers = 2;
    let mut handles = Vec::new();

    for _ in 0..num_consumers {
      let stream_clone = new_stream.clone();
      let handle = tokio::spawn(async move {
        let mut results = Vec::new();
        while let Ok(Some(value)) = stream_clone.next().await {
          results.push(value);
        }
        results
      });
      handles.push(handle);
    }

    // Now start producing values
    let producer = tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(1)).await;
      }
      stream_clone.close().await.unwrap();
    });

    // Wait for producer to finish
    producer.await.unwrap();

    // Collect results from all consumers
    let mut all_results = Vec::new();
    for handle in handles {
      let results = handle.await.unwrap();
      all_results.extend(results);
    }

    // Sort results for deterministic comparison
    all_results.sort();

    // Each consumer should see all values
    assert_eq!(all_results, vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
  }
}
