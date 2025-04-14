use effect_stream::{EffectError, EffectResult, EffectStream, EffectStreamOperator};
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
        let mut current_retry = 0;
        let mut last_error = None;
        let mut current_stream = stream_clone.clone();

        loop {
          match current_stream.next().await {
            Ok(Some(item)) => {
              // On successful item, reset retry count and last error
              current_retry = 0;
              last_error = None;
              if (new_stream_clone.push(item).await).is_err() {
                break;
              }
            }
            Ok(None) => {
              // Stream completed successfully
              break;
            }
            Err(err) => {
              // Store the last error
              last_error = Some(err.clone());

              // On error, retry if we haven't exceeded max retries
              if current_retry < max_retries {
                current_retry += 1;
                tokio::time::sleep(backoff).await;
                // Create a fresh stream for retry
                current_stream = stream_clone.clone();
                continue;
              } else {
                // Propagate the last error before breaking
                if let Some(err) = last_error {
                  match err {
                    EffectError::Processing(e)
                    | EffectError::Read(e)
                    | EffectError::Write(e)
                    | EffectError::Custom(e) => {
                      new_stream_clone.set_error(e).await.unwrap();
                    }
                    EffectError::Closed => {
                      new_stream_clone.close().await.unwrap();
                    }
                  }
                }
                break;
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
  use tokio::sync::Mutex;

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

    // Create a stream that will fail on first attempt but succeed on retry
    let error_count = Arc::new(Mutex::new(0));
    let error_count_clone = error_count.clone();

    tokio::spawn(async move {
      let mut count = error_count_clone.lock().await;
      if *count == 0 {
        *count += 1;
        stream_clone
          .set_error(TestError("First attempt error".to_string()))
          .await
          .unwrap();
      } else {
        for i in 1..=3 {
          stream_clone.push(i).await.unwrap();
        }
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
  async fn test_retry_max_attempts() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone
        .set_error(TestError("Persistent error".to_string()))
        .await
        .unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = RetryOperator::new(2, Duration::from_millis(10));
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

    // Create a stream that will fail on first attempt but succeed on retry
    let error_count = Arc::new(Mutex::new(0));
    let error_count_clone = error_count.clone();

    // Create the operator and transform stream first
    let operator = RetryOperator::new(3, Duration::from_millis(10));
    let new_stream = operator.transform(stream).await.unwrap();

    // Create multiple consumer tasks before producing any values
    let num_consumers = 2;
    let results = Arc::new(Mutex::new(Vec::new()));

    // Start consumers first
    let mut consumer_handles = Vec::new();
    for _ in 0..num_consumers {
      let stream_clone = new_stream.clone();
      let results_clone = results.clone();
      let handle = tokio::spawn(async move {
        let mut local_results = Vec::new();
        while let Ok(Some(value)) = stream_clone.next().await {
          local_results.push(value);
        }
        let mut results = results_clone.lock().await;
        results.extend(local_results);
      });
      consumer_handles.push(handle);
    }

    // Now start producing values
    let producer = tokio::spawn(async move {
      let mut count = error_count_clone.lock().await;
      if *count == 0 {
        *count += 1;
        stream_clone
          .set_error(TestError("First attempt error".to_string()))
          .await
          .unwrap();
      } else {
        for i in 1..=3 {
          stream_clone.push(i).await.unwrap();
        }
      }
      stream_clone.close().await.unwrap();
    });

    // Wait for producer to finish
    producer.await.unwrap();

    // Wait for all consumers to finish
    for handle in consumer_handles {
      handle.await.unwrap();
    }

    // Get the results
    let mut all_results = results.lock().await;
    all_results.sort();

    // Each consumer should see all values
    assert_eq!(*all_results, vec![1, 1, 2, 2, 3, 3]);
  }
}
