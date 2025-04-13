use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub struct FlattenOperator<T>
where
  T: Send + Sync + 'static,
{
  _phantom: std::marker::PhantomData<T>,
}

impl<T> FlattenOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T> Default for FlattenOperator<T>
where
  T: Send + Sync + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E> EffectStreamOperator<Vec<T>, E, T> for FlattenOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<Vec<T>, E>) -> Self::Future {
    let stream_clone = stream.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(items)) = stream_clone.next().await {
          for item in items {
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

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_flatten_basic() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![1, 2, 3]).await.unwrap();
      stream_clone.push(vec![4, 5]).await.unwrap();
      stream_clone.push(vec![6]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
  }

  #[tokio::test]
  async fn test_flatten_empty_input() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_flatten_mixed_input() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.push(vec![1]).await.unwrap();
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.push(vec![2, 3]).await.unwrap();
      stream_clone.push(vec![]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = FlattenOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_flatten_concurrent() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = FlattenOperator::new();
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
      stream_clone.push(vec![1, 2]).await.unwrap();
      tokio::time::sleep(Duration::from_millis(1)).await;
      stream_clone.push(vec![3, 4]).await.unwrap();
      tokio::time::sleep(Duration::from_millis(1)).await;
      stream_clone.push(vec![5]).await.unwrap();
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
