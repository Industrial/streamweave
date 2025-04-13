use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio::time::Duration;

pub struct RetryOperator<T>
where
  T: Send + Sync + 'static,
{
  max_retries: usize,
  backoff: Duration,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> RetryOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(max_retries: usize, backoff: Duration) -> Self {
    Self {
      max_retries,
      backoff,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for RetryOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let max_retries = self.max_retries;
    let backoff = self.backoff;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          let mut retries = 0;
          let mut success = false;

          while retries < max_retries && !success {
            match new_stream_clone.push(item.clone()).await {
              Ok(_) => success = true,
              Err(_) => {
                retries += 1;
                if retries < max_retries {
                  tokio::time::sleep(backoff).await;
                }
              }
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
  use std::sync::Arc;
  use tokio::{sync::Mutex, time::Duration};

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_retry_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = RetryOperator::new(3, Duration::from_millis(10));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_retry_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = RetryOperator::new(3, Duration::from_millis(10));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_retry_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = RetryOperator::new(3, Duration::from_millis(10));
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
      for i in 1..=3 {
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
    assert_eq!(all_results, vec![1, 1, 2, 2, 3, 3]);
  }
}
