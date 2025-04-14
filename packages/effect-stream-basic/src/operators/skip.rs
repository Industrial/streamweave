use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio;
use tokio::sync::Mutex;

pub struct SkipOperator<T>
where
  T: Send + Sync + 'static,
{
  skip: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> SkipOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(skip: usize) -> Self {
    Self {
      skip,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for SkipOperator<T>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let skip = self.skip;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut count = 0;
        while let Ok(Some(item)) = stream_clone.next().await {
          if count >= skip {
            new_stream_clone.push(item).await.unwrap();
          }
          count += 1;
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
  async fn test_skip_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = SkipOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![3, 4, 5]);
  }

  #[tokio::test]
  async fn test_skip_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = SkipOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_skip_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = SkipOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    // Create multiple consumer tasks before producing any values
    let num_consumers = 2;
    let mut handles = Vec::new();
    let results = Arc::new(Mutex::new(Vec::new()));

    // Now start producing values
    let producer = tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    // Start consumers after producer to ensure they don't miss any values
    for _ in 0..num_consumers {
      let stream_clone = new_stream.clone();
      let results_clone = results.clone();
      let handle = tokio::spawn(async move {
        let mut local_results = Vec::new();
        while let Ok(Some(value)) = stream_clone.next().await {
          local_results.push(value);
          tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let mut results = results_clone.lock().await;
        results.extend(local_results);
      });
      handles.push(handle);
    }

    // Wait for producer to finish
    producer.await.unwrap();

    // Give consumers time to process any remaining items
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Wait for all consumers to finish
    for handle in handles {
      handle.await.unwrap();
    }

    // Get the results
    let mut all_results = results.lock().await;
    all_results.sort();

    // Each consumer should see the same values after skipping the first 2
    assert_eq!(*all_results, vec![3, 3, 4, 4, 5, 5]);
  }
}
