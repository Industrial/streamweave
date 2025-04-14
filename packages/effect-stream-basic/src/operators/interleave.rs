use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use futures::{Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct InterleaveOperator<T>
where
  T: Send + Sync + 'static,
{
  other: Arc<Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>>,
}

impl<T> InterleaveOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(other: Pin<Box<dyn Stream<Item = T> + Send>>) -> Self {
    Self {
      other: Arc::new(Mutex::new(other)),
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for InterleaveOperator<T>
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
        let mut other_guard = other.lock().await;
        let mut stream1_items = Vec::new();
        let mut stream2_items = Vec::new();
        let mut stream1_done = false;
        let mut stream2_done = false;
        let mut idx = 0;

        // First, collect all items from both streams
        while !stream1_done || !stream2_done {
          if !stream1_done {
            match stream_clone.next().await {
              Ok(Some(item)) => stream1_items.push(item),
              _ => stream1_done = true,
            }
          }

          if !stream2_done {
            match other_guard.next().await {
              Some(item) => stream2_items.push(item),
              None => stream2_done = true,
            }
          }
        }

        // Then interleave them
        while idx < stream1_items.len() || idx < stream2_items.len() {
          if idx < stream1_items.len() {
            new_stream_clone
              .push(stream1_items[idx].clone())
              .await
              .unwrap();
          }
          if idx < stream2_items.len() {
            new_stream_clone
              .push(stream2_items[idx].clone())
              .await
              .unwrap();
          }
          idx += 1;
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
  async fn test_interleave_basic() {
    let other = stream::iter(vec![4, 5, 6].into_iter());
    let operator = InterleaveOperator::new(Box::pin(other));

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

    assert_eq!(results, vec![1, 4, 2, 5, 3, 6]);
  }

  #[tokio::test]
  async fn test_interleave_empty_input() {
    let other = stream::iter(Vec::<i32>::new());
    let operator = InterleaveOperator::new(Box::pin(other));

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
  async fn test_interleave_concurrent() {
    let other = stream::iter(vec![4, 5, 6].into_iter());
    let operator = InterleaveOperator::new(Box::pin(other));

    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let new_stream = operator.transform(stream).await.unwrap();

    // Create multiple consumer tasks before producing any values
    let num_consumers = 2;
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
      stream_clone.push(1).await.unwrap();
      tokio::time::sleep(Duration::from_millis(10)).await;
      stream_clone.push(2).await.unwrap();
      tokio::time::sleep(Duration::from_millis(10)).await;
      stream_clone.push(3).await.unwrap();
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

    // Each consumer should see all values
    assert_eq!(*all_results, vec![1, 2, 3, 4, 5, 6]);
  }

  #[tokio::test]
  async fn test_interleave_uneven() {
    let other = stream::iter(vec![4, 5].into_iter());
    let operator = InterleaveOperator::new(Box::pin(other));

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

    assert_eq!(results, vec![1, 4, 2, 5, 3]);
  }
}
