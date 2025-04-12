use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use futures::{Stream, StreamExt};
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
    let stream_clone = stream.clone();
    let duration = self.duration;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let mut new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        while let Ok(Some(item)) = stream_clone.next().await {
          sleep(duration).await;
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
  use tokio::time::Instant;

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
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DelayOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let start = Instant::now();
    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }
    let duration = start.elapsed();

    assert_eq!(results, vec![1, 2, 3, 4, 5]);
    assert!(duration >= Duration::from_millis(500)); // Each item delayed by 100ms
  }

  #[tokio::test]
  async fn test_delay_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = DelayOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_delay_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = DelayOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results1 = Vec::new();
    let mut results2 = Vec::new();
    let mut new_stream_clone1 = new_stream.clone();
    let mut new_stream_clone2 = new_stream.clone();

    let handle1 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream_clone1.next().await {
        results1.push(value);
      }
      results1
    });

    let handle2 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream_clone2.next().await {
        results2.push(value);
      }
      results2
    });

    let (results1, results2) = tokio::join!(handle1, handle2);
    let results1 = results1.unwrap();
    let results2 = results2.unwrap();

    assert_eq!(results1, vec![1, 2, 3, 4, 5]);
    assert_eq!(results2, vec![1, 2, 3, 4, 5]);
  }
}
