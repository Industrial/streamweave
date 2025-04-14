use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::sleep;

pub struct DelayOperator<T>
where
  T: Send + Sync + 'static,
{
  duration: Duration,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> DelayOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(duration: Duration) -> Self {
    Self {
      duration,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for DelayOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let duration = self.duration;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();
      let stream_clone = stream.clone();

      tokio::spawn(async move {
        let mut buffer = Vec::new();

        // First, collect all items
        while let Ok(Some(item)) = stream_clone.next().await {
          buffer.push(item);
        }

        // Then, push them with delays
        for item in buffer {
          sleep(duration).await;
          if let Err(_) = new_stream_clone.push(item).await {
            break;
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
  async fn test_delay_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = DelayOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream.clone()).await.unwrap();

    let producer = tokio::spawn(async move {
      for i in 1..=5 {
        stream.push(i).await.unwrap();
      }
      stream.close().await.unwrap();
    });

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    producer.await.unwrap();
    assert_eq!(results, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_delay_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = DelayOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream.clone()).await.unwrap();

    stream.close().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_delay_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = DelayOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream.clone()).await.unwrap();

    let results = Arc::new(Mutex::new(Vec::new()));

    // Start producer task first
    let producer = tokio::spawn(async move {
      for i in 1..=5 {
        stream.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream.close().await.unwrap();
    });

    // Wait for producer to finish
    producer.await.unwrap();

    // Now start consumer
    let consumer = {
      let new_stream = new_stream.clone();
      let results = results.clone();
      tokio::spawn(async move {
        let mut local_results = Vec::new();
        while let Ok(Some(value)) = new_stream.next().await {
          local_results.push(value);
        }
        *results.lock().await = local_results;
      })
    };

    // Wait for all delayed values to be processed
    tokio::time::sleep(Duration::from_millis(600)).await;

    // Wait for consumer with timeout
    let timeout = Duration::from_secs(2);
    let consumer_result = tokio::time::timeout(timeout, consumer).await;

    assert!(consumer_result.is_ok(), "Consumer timed out");

    // Get results
    let results = results.lock().await;
    assert_eq!(*results, vec![1, 2, 3, 4, 5]);
  }
}
