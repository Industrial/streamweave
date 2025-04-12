use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct MergeOperator<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  _phantom: std::marker::PhantomData<T>,
  streams: Vec<EffectStream<T, E>>,
}

impl<T, E> MergeOperator<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
      streams: Vec::new(),
    }
  }

  pub fn add_stream(&mut self, stream: EffectStream<T, E>) {
    self.streams.push(stream);
  }
}

impl<T, E> Default for MergeOperator<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  fn default() -> Self {
    Self::new()
  }
}

impl<T, E> EffectStreamOperator<T, E, T> for MergeOperator<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let mut streams = self.streams.clone();
    streams.push(stream_clone);

    Box::pin(async move {
      let new_stream = EffectStream::<T, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut handles = Vec::new();
        for stream in streams {
          let new_stream_clone = new_stream_clone.clone();
          handles.push(tokio::spawn(async move {
            while let Ok(Some(item)) = stream.next().await {
              new_stream_clone.push(item).await.unwrap();
            }
          }));
        }

        for handle in handles {
          handle.await.unwrap();
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
  async fn test_merge_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = MergeOperator::<i32, TestError>::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_merge_multiple_streams() {
    let stream1 = EffectStream::<i32, TestError>::new();
    let stream1_clone = stream1.clone();
    let stream2 = EffectStream::<i32, TestError>::new();
    let stream2_clone = stream2.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream1_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream1_clone.close().await.unwrap();
    });

    tokio::spawn(async move {
      for i in 4..=6 {
        stream2_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream2_clone.close().await.unwrap();
    });

    let mut operator = MergeOperator::<i32, TestError>::new();
    operator.add_stream(stream2);
    let new_stream = operator.transform(stream1).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    results.sort();
    assert_eq!(results, vec![1, 2, 3, 4, 5, 6]);
  }

  #[tokio::test]
  async fn test_merge_concurrent() {
    let stream1 = EffectStream::<i32, TestError>::new();
    let stream1_clone = stream1.clone();
    let stream2 = EffectStream::<i32, TestError>::new();
    let stream2_clone = stream2.clone();

    tokio::spawn(async move {
      for i in 1..=3 {
        stream1_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream1_clone.close().await.unwrap();
    });

    tokio::spawn(async move {
      for i in 4..=6 {
        stream2_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream2_clone.close().await.unwrap();
    });

    let mut operator = MergeOperator::<i32, TestError>::new();
    operator.add_stream(stream2);
    let new_stream = operator.transform(stream1).await.unwrap();
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

    handle1.await.unwrap();
    handle2.await.unwrap();

    let mut final_results = results.lock().await;
    final_results.sort();
    assert_eq!(*final_results, vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6]);
  }
}
