use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

pub struct RateLimitOperator<T>
where
  T: Send + Sync + 'static,
{
  rate_limit: usize,
  time_window: Duration,
  count: Arc<AtomicUsize>,
  window_start: Arc<tokio::sync::RwLock<Instant>>,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> RateLimitOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      rate_limit,
      time_window,
      count: Arc::new(AtomicUsize::new(0)),
      window_start: Arc::new(tokio::sync::RwLock::new(Instant::now())),
      _phantom: std::marker::PhantomData,
    }
  }

  #[allow(dead_code)]
  async fn check_rate_limit(&self) -> bool {
    let now = Instant::now();
    let mut window_start = self.window_start.write().await;

    if now.duration_since(*window_start) >= self.time_window {
      self.count.store(0, Ordering::SeqCst);
      *window_start = now;
    }

    if self.count.load(Ordering::SeqCst) >= self.rate_limit {
      false
    } else {
      self.count.fetch_add(1, Ordering::SeqCst);
      true
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for RateLimitOperator<T>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let count = Arc::clone(&self.count);
    let window_start = Arc::clone(&self.window_start);
    let rate_limit = self.rate_limit;
    let time_window = self.time_window;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          let now = Instant::now();
          let mut window_start = window_start.write().await;

          if now.duration_since(*window_start) > time_window {
            count.store(0, Ordering::SeqCst);
            *window_start = now;
          }

          if count.load(Ordering::SeqCst) < rate_limit {
            count.fetch_add(1, Ordering::SeqCst);
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
  use tokio::{sync::Mutex, time::sleep};

  use super::*;

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_rate_limit_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = RateLimitOperator::new(3, Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_rate_limit_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = RateLimitOperator::new(3, Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_rate_limit_actual_rate_limit() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = RateLimitOperator::new(2, Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2]);
  }

  #[tokio::test]
  async fn test_rate_limit_reset() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=2 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = RateLimitOperator::new(2, Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2]);

    sleep(Duration::from_millis(200)).await;

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 3..=4 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![3, 4]);
  }

  #[tokio::test]
  async fn test_rate_limit_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = RateLimitOperator::new(3, Duration::from_millis(100));
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
      for i in 1..=6 {
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

    // Each consumer should see the rate-limited values
    assert_eq!(all_results, vec![1, 1, 2, 2, 3, 3]);
  }
}
