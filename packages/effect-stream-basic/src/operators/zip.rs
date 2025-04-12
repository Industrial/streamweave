use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct ZipOperator<T>
where
  T: Send + Sync + 'static,
{
  _phantom: std::marker::PhantomData<T>,
}

impl<T> ZipOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new() -> Self {
    Self {
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<Vec<T>, E, Vec<T>> for ZipOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + 'static + std::fmt::Debug,
{
  type Future =
    Pin<Box<dyn Future<Output = EffectResult<EffectStream<Vec<T>, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<Vec<T>, E>) -> Self::Future {
    let stream_clone = stream.clone();

    Box::pin(async move {
      let new_stream = EffectStream::<Vec<T>, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut buffers: Vec<Vec<T>> = Vec::new();

        while let Ok(Some(items)) = stream_clone.next().await {
          buffers.push(items);
        }

        // Early return if no input
        if buffers.is_empty() {
          new_stream_clone.close().await.unwrap();
          return;
        }

        // Get the length of the longest vector
        let max_len = buffers.iter().map(|v| v.len()).max().unwrap_or(0);

        // Yield transposed vectors
        for i in 0..max_len {
          let mut result = Vec::new();
          for buffer in &buffers {
            if let Some(item) = buffer.get(i) {
              result.push(item.clone());
            }
          }
          if !result.is_empty() {
            new_stream_clone.push(result).await.unwrap();
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

  use tokio::sync::Mutex;

  use super::*;

  #[derive(Debug, Clone)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  #[tokio::test]
  async fn test_zip_basic() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![1, 2, 3]).await.unwrap();
      stream_clone.push(vec![4, 5, 6]).await.unwrap();
      stream_clone.push(vec![7, 8, 9]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = ZipOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![vec![1, 4, 7], vec![2, 5, 8], vec![3, 6, 9]]);
  }

  #[tokio::test]
  async fn test_zip_empty_input() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = ZipOperator::new();
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_zip_concurrent() {
    let stream = EffectStream::<Vec<i32>, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(vec![1, 2, 3]).await.unwrap();
      stream_clone.push(vec![4, 5, 6]).await.unwrap();
      stream_clone.push(vec![7, 8, 9]).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let operator = ZipOperator::new();
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

    handle1.await.unwrap();
    handle2.await.unwrap();

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert!(final_results.contains(&vec![1, 4, 7]));
    assert!(final_results.contains(&vec![2, 5, 8]));
    assert!(final_results.contains(&vec![3, 6, 9]));
  }
}
