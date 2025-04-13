use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio;

pub struct LimitOperator<T>
where
  T: Send + Sync + 'static,
{
  limit: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> LimitOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(limit: usize) -> Self {
    Self {
      limit,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for LimitOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let limit = self.limit;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut count = 0;
        let stream_clone = stream_clone;

        while count < limit {
          match stream_clone.next().await {
            Ok(Some(item)) => {
              new_stream_clone.push(item).await.unwrap();
              count += 1;
            }
            Ok(None) => break,
            Err(_) => break,
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

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_limit_basic() {
    let operator = LimitOperator::new(3);

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.push(4).await.unwrap();
      stream_clone.push(5).await.unwrap();
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
  async fn test_limit_empty_input() {
    let operator = LimitOperator::new(3);

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
  async fn test_limit_concurrent() {
    let operator = LimitOperator::new(3);

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
      stream_clone.push(1).await.unwrap();
      tokio::time::sleep(Duration::from_millis(1)).await;
      stream_clone.push(2).await.unwrap();
      tokio::time::sleep(Duration::from_millis(1)).await;
      stream_clone.push(3).await.unwrap();
      tokio::time::sleep(Duration::from_millis(1)).await;
      stream_clone.push(4).await.unwrap();
      tokio::time::sleep(Duration::from_millis(1)).await;
      stream_clone.push(5).await.unwrap();
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

    // Each consumer should see the first 3 values
    assert_eq!(all_results, vec![1, 1, 2, 2, 3, 3]);
  }

  #[tokio::test]
  async fn test_limit_zero() {
    let operator = LimitOperator::new(0);

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

    assert_eq!(results, Vec::<i32>::new());
  }
}
