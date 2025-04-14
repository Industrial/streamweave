use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct FlatMapOperator<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: Send + Sync + 'static,
  O: Send + Sync + 'static,
{
  f: F,
  _phantom_i: std::marker::PhantomData<I>,
  _phantom_o: std::marker::PhantomData<O>,
}

impl<F, I, O> FlatMapOperator<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: Send + Sync + 'static,
  O: Send + Sync + 'static,
{
  pub fn new(f: F) -> Self {
    Self {
      f,
      _phantom_i: std::marker::PhantomData,
      _phantom_o: std::marker::PhantomData,
    }
  }
}

impl<F, I, O, E> EffectStreamOperator<I, E, O> for FlatMapOperator<F, I, O>
where
  F: Fn(I) -> Vec<O> + Send + Clone + 'static,
  I: Send + Sync + 'static,
  O: Send + Sync + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<O, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<I, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let f = self.f.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<O, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          let mapped_items = f(item);
          for mapped_item in mapped_items {
            new_stream_clone.push(mapped_item).await.unwrap();
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
  async fn test_flat_map_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = FlatMapOperator::new(|x: i32| vec![x * 2, x * 3]);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![2, 3, 4, 6, 6, 9]);
  }

  #[tokio::test]
  async fn test_flat_map_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = FlatMapOperator::new(|_: i32| Vec::<i32>::new());
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_flat_map_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = FlatMapOperator::new(|x: i32| vec![x * 2, x * 3]);
    let new_stream = operator.transform(stream).await.unwrap();

    // Start producing values first
    let producer = tokio::spawn(async move {
      for i in 1..=3 {
        if let Err(e) = stream_clone.push(i).await {
          eprintln!("Error pushing value: {}", e);
          break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      if let Err(e) = stream_clone.close().await {
        eprintln!("Error closing stream: {}", e);
      }
    });

    // Create multiple consumer tasks
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

    // Wait for producer to finish with timeout
    tokio::select! {
      result = producer => {
        result.expect("Producer task failed");
      }
      _ = tokio::time::sleep(Duration::from_secs(1)) => {
        panic!("Producer timed out");
      }
    }

    // Collect results from all consumers with timeout
    let mut all_results = Vec::new();
    for handle in handles {
      match tokio::time::timeout(Duration::from_secs(1), handle).await {
        Ok(Ok(results)) => all_results.extend(results),
        Ok(Err(e)) => panic!("Consumer task failed: {}", e),
        Err(_) => panic!("Consumer timed out"),
      }
    }

    // Sort results for deterministic comparison
    all_results.sort();

    // Each consumer should see some of the mapped values
    // The total number of values should be between 6 (all values seen by one consumer)
    // and 12 (all values seen by both consumers)
    assert!(
      all_results.len() >= 6 && all_results.len() <= 12,
      "Expected between 6 and 12 values, got {}",
      all_results.len()
    );

    // All values should be valid mapped values
    let valid_values = vec![2, 3, 4, 6, 6, 9];
    for value in &all_results {
      assert!(valid_values.contains(value), "Unexpected value: {}", value);
    }
  }
}
