use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct BufferOperator<T>
where
  T: Send + Sync + 'static,
{
  capacity: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> BufferOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(capacity: usize) -> Self {
    Self {
      capacity,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for BufferOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let capacity = self.capacity;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut buffer = Vec::with_capacity(capacity);
        while let Ok(Some(item)) = stream_clone.next().await {
          buffer.push(item);
          if buffer.len() >= capacity {
            for item in buffer.drain(..) {
              new_stream_clone.push(item).await.unwrap();
            }
          }
        }
        // Flush remaining items
        for item in buffer.drain(..) {
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
  use std::sync::Arc;

  use super::*;
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
  async fn test_buffer_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = BufferOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_buffer_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = BufferOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_buffer_async() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = BufferOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_buffer_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    // Create the operator and transform stream first
    let operator = BufferOperator::new(2);
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
      for i in 1..=5 {
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
    assert_eq!(all_results, vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5]);
  }
}
