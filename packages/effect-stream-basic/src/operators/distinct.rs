use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  seen: Arc<Mutex<HashSet<T>>>,
}

impl<T> DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  pub fn new() -> Self {
    Self {
      seen: Arc::new(Mutex::new(HashSet::new())),
    }
  }
}

impl<T> Default for DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for DistinctOperator<T>
where
  T: Send + Sync + Hash + Eq + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let new_stream = EffectStream::new();
    let new_stream_clone = new_stream.clone();
    let seen = Arc::clone(&self.seen);

    Box::pin(async move {
      let stream_clone = stream.clone();

      tokio::spawn(async move {
        let mut items = Vec::new();
        let mut seen_guard = seen.lock().await;

        // First, collect all items and filter out duplicates
        while let Ok(Some(item)) = stream_clone.next().await {
          if seen_guard.insert(item.clone()) {
            items.push(item);
          }
        }

        // Then push all items to the new stream
        for item in items {
          new_stream_clone.push(item).await.unwrap();
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
  use std::time::Duration;

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_distinct_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in [1, 2, 2, 3, 3, 3] {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DistinctOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_distinct_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = DistinctOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_distinct_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = DistinctOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    // Create multiple consumer tasks before producing any values
    let num_consumers = 3;
    let mut handles = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));

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
      handles.push(handle);
    }

    // Now start producing values
    let producer = tokio::spawn(async move {
      for i in [1, 2, 2, 3, 3, 3] {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
      }
      stream_clone.close().await.unwrap();
    });

    // Wait for producer to finish
    producer.await.unwrap();

    // Give consumers time to process any remaining items
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Wait for all consumers to finish with timeout
    for handle in handles {
      tokio::select! {
          result = handle => result.unwrap(),
          _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("Consumer timed out"),
      }
    }

    // Get the results
    let mut all_results = results.lock().await;
    all_results.sort();

    // Each consumer should see only the distinct values
    assert_eq!(*all_results, vec![1, 2, 3]);
  }
}
