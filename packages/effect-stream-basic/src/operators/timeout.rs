use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;
use tokio::time::{timeout, Duration};

pub struct TimeoutOperator<T>
where
  T: Send + Sync + 'static,
{
  duration: Duration,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> TimeoutOperator<T>
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

impl<T, E> EffectStreamOperator<T, E, T> for TimeoutOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static + std::fmt::Debug,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let duration = self.duration;

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        loop {
          match timeout(duration, stream_clone.next()).await {
            Ok(Ok(Some(value))) => {
              new_stream_clone.push(value).await.unwrap();
            }
            Ok(Ok(None)) => break,
            Ok(Err(_)) => break,
            Err(_) => break,
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
  use std::sync::Arc;

  use super::*;
  use tokio::{sync::Mutex, time::sleep};

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_timeout_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = TimeoutOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_timeout_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = TimeoutOperator::new(Duration::from_millis(100));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_timeout_actual_timeout() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        sleep(Duration::from_millis(100)).await;
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = TimeoutOperator::new(Duration::from_millis(50));
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert!(results.is_empty());
  }

  #[tokio::test]
  async fn test_timeout_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    let producer = tokio::spawn(async move {
      for i in 1..=3 {
        stream_clone.push(i).await.unwrap();
        sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = TimeoutOperator::new(Duration::from_millis(200));
    let new_stream = operator.transform(stream).await.unwrap();
    let new_stream_clone = new_stream.clone();

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone1 = results.clone();
    let results_clone2 = results.clone();

    let handle1 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream.next().await {
        results_clone1.lock().await.push(value);
      }
    });

    let handle2 = tokio::spawn(async move {
      while let Ok(Some(value)) = new_stream_clone.next().await {
        results_clone2.lock().await.push(value);
      }
    });

    // Wait for producer to finish
    producer.await.unwrap();

    // Wait for consumers with timeout
    tokio::select! {
      _ = handle1 => {},
      _ = handle2 => {},
      _ = sleep(Duration::from_secs(5)) => panic!("Test timed out waiting for consumers"),
    }

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert!(final_results.contains(&1));
    assert!(final_results.contains(&2));
    assert!(final_results.contains(&3));
  }
}
