use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use futures::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ConcatOperator<T>
where
  T: Send + Sync + 'static,
{
  other: Arc<Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>>,
}

impl<T> ConcatOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      other: Arc::new(Mutex::new(other)),
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for ConcatOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(
    &self,
    stream: EffectStream<T, E>,
  ) -> Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>> {
    let new_stream = EffectStream::new();
    let new_stream_clone = new_stream.clone();
    let other = Arc::clone(&self.other);

    Box::pin(async move {
      let stream_clone = stream.clone();

      tokio::spawn(async move {
        let stream_clone = stream_clone;
        let mut items = Vec::new();

        // First, collect all items from the first stream
        while let Ok(Some(item)) = stream_clone.next().await {
          items.push(item);
        }

        // Then, collect all items from the second stream
        let mut other_clone = other.lock().await;
        while let Some(item) = other_clone.next().await {
          items.push(item);
        }
        drop(other_clone);

        // Finally, push all items to the new stream
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
  use futures::stream;
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
  async fn test_concat_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let other = stream::iter(vec![4, 5, 6].into_iter());
    let operator = ConcatOperator::new(Box::pin(other));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
  }

  #[tokio::test]
  async fn test_concat_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let other = stream::iter(Vec::<i32>::new());
    let operator = ConcatOperator::new(Box::pin(other));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_concat_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create multiple consumer tasks
    let num_consumers = 2;
    let mut handles = Vec::new();
    let results = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Create the operator and transform stream
    let other = stream::iter(vec![4, 5, 6].into_iter());
    let operator = ConcatOperator::new(Box::pin(other));
    let new_stream = operator.transform(stream).await.unwrap();

    // Start producing values
    let producer = tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    // Start consumers
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

    // Each consumer should see all values from both streams
    assert_eq!(*all_results, vec![1, 2, 3, 4, 5, 6]);
  }
}
