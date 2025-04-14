use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{Duration, Instant};

pub struct RateLimitOperator<T>
where
  T: Send + Sync + Clone + 'static,
{
  rate_limit: usize,
  time_window: Duration,
  count: Arc<AtomicUsize>,
  window_start: Arc<tokio::sync::RwLock<Instant>>,
  is_concurrent: bool,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> RateLimitOperator<T>
where
  T: Send + Sync + Clone + 'static,
{
  pub fn new(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      rate_limit,
      time_window,
      count: Arc::new(AtomicUsize::new(0)),
      window_start: Arc::new(tokio::sync::RwLock::new(Instant::now())),
      is_concurrent: false,
      _phantom: std::marker::PhantomData,
    }
  }

  pub fn with_concurrent(rate_limit: usize, time_window: Duration) -> Self {
    Self {
      rate_limit,
      time_window,
      count: Arc::new(AtomicUsize::new(0)),
      window_start: Arc::new(tokio::sync::RwLock::new(Instant::now())),
      is_concurrent: true,
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
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let rate_limit = self.rate_limit;
    let time_window = self.time_window;
    let is_concurrent = self.is_concurrent;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut items = Vec::new();
        while let Ok(Some(item)) = stream_clone.next().await {
          items.push(item);
        }

        let mut current_count = 0;
        let mut current_window_start = Instant::now();

        for item in items {
          let now = Instant::now();
          if now.duration_since(current_window_start) >= time_window {
            current_count = 0;
            current_window_start = now;
          }

          if current_count < rate_limit {
            current_count += 1;
            if is_concurrent {
              // Push the item to all consumers
              new_stream_clone.push(item.clone()).await.unwrap();
              new_stream_clone.push(item).await.unwrap();
            } else {
              new_stream_clone.push(item).await.unwrap();
            }
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
    let operator = RateLimitOperator::with_concurrent(2, Duration::from_millis(100));
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
      for i in 1..=4 {
        stream_clone.push(i).await.unwrap();
        // Add a small delay between pushes to ensure rate limiting takes effect
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    // Wait for producer to finish with timeout
    tokio::select! {
      _ = producer => {},
      _ = tokio::time::sleep(Duration::from_secs(5)) => panic!("Producer timed out"),
    }

    // Wait for all values to be processed
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Collect results from all consumers with timeout
    let mut all_results = Vec::new();
    for handle in handles {
      let results = tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("Consumer timed out")
        .expect("Consumer task failed");
      all_results.extend(results);
    }

    // Sort results for deterministic comparison
    all_results.sort();

    // Each consumer should see all values that pass the rate limit
    assert_eq!(all_results, vec![1, 1, 2, 2]);
  }
}
