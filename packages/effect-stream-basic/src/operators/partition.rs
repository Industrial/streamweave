use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct PartitionOperator<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: Send + Sync + 'static,
{
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
}

impl<F, T> PartitionOperator<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: Send + Sync + 'static,
{
  pub fn new(predicate: F) -> Self {
    Self {
      predicate,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<F, T, E> EffectStreamOperator<T, E, (Vec<T>, Vec<T>)> for PartitionOperator<F, T>
where
  F: Fn(&T) -> bool + Send + Clone + 'static,
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static + std::fmt::Debug,
{
  type Future = Pin<
    Box<dyn Future<Output = EffectResult<EffectStream<(Vec<T>, Vec<T>), E>, E>> + Send + 'static>,
  >;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let predicate = self.predicate.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<(Vec<T>, Vec<T>), E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut matches = Vec::new();
        let mut non_matches = Vec::new();

        while let Ok(Some(item)) = stream_clone.next().await {
          if predicate(&item) {
            matches.push(item);
          } else {
            non_matches.push(item);
          }
        }

        if !matches.is_empty() || !non_matches.is_empty() {
          new_stream_clone.push((matches, non_matches)).await.unwrap();
        }
        new_stream_clone.close().await.unwrap();
      });

      Ok(new_stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;
  use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
  };

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_partition_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=6 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = PartitionOperator::new(|x: &i32| x % 2 == 0);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![(vec![2, 4, 6], vec![1, 3, 5])]);
  }

  #[tokio::test]
  async fn test_partition_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = PartitionOperator::new(|x: &i32| x % 2 == 0);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::new());
  }

  #[tokio::test]
  async fn test_partition_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = PartitionOperator::new(|x: &i32| x % 2 == 0);
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

    // Each consumer should see the partitioned values
    assert_eq!(all_results.len(), 2);
    assert!(all_results.contains(&(vec![2, 4, 6], vec![1, 3, 5])));
  }
}
