use effect_stream::{EffectResult, EffectStream, EffectStreamOperator};
use std::future::Future;
use std::pin::Pin;
use tokio;

pub struct ChunkOperator<T>
where
  T: Send + Sync + 'static,
{
  size: usize,
  _phantom: std::marker::PhantomData<T>,
}

impl<T> ChunkOperator<T>
where
  T: Send + Sync + 'static,
{
  pub fn new(size: usize) -> Self {
    Self {
      size,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, E> EffectStreamOperator<T, E, Vec<T>> for ChunkOperator<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + std::fmt::Debug + 'static,
{
  type Future =
    Pin<Box<dyn Future<Output = EffectResult<EffectStream<Vec<T>, E>, E>> + Send + 'static>>;

  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
    let stream_clone = stream.clone();
    let size = self.size;

    Box::pin(async move {
      let new_stream = EffectStream::<Vec<T>, E>::new();
      let new_stream_clone = new_stream.clone();

      tokio::spawn(async move {
        let mut current_chunk = Vec::with_capacity(size);
        while let Ok(Some(item)) = stream_clone.next().await {
          current_chunk.push(item);
          if current_chunk.len() >= size {
            new_stream_clone.push(current_chunk).await.unwrap();
            current_chunk = Vec::with_capacity(size);
          }
        }
        if !current_chunk.is_empty() {
          new_stream_clone.push(current_chunk).await.unwrap();
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
  async fn test_chunk_basic() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ChunkOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![vec![1, 2], vec![3, 4], vec![5]]);
  }

  #[tokio::test]
  async fn test_chunk_empty_input() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let operator = ChunkOperator::new(2);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<Vec<i32>>::new());
  }

  #[tokio::test]
  async fn test_chunk_concurrent() {
    let stream = EffectStream::<i32, TestError>::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
      for i in 1..=5 {
        stream_clone.push(i).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
      }
      stream_clone.close().await.unwrap();
    });

    let operator = ChunkOperator::new(2);
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
    assert_eq!(final_results.len(), 6); // Each chunk is received twice due to cloning
    assert!(final_results.contains(&vec![1, 2]));
    assert!(final_results.contains(&vec![3, 4]));
    assert!(final_results.contains(&vec![5]));
  }
}
