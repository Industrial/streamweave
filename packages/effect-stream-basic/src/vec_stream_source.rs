use effect_stream::{EffectResult, EffectStream, EffectStreamSource};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A stream that produces values from a vector
pub struct VecStreamSource<T>
where
  T: Send + Sync + Clone + 'static,
{
  vec: Arc<Mutex<Vec<T>>>,
  index: Arc<Mutex<usize>>,
}

impl<T> VecStreamSource<T>
where
  T: Send + Sync + Clone + 'static,
{
  /// Create a new stream from a vector
  pub fn new(vec: Vec<T>) -> Self {
    Self {
      vec: Arc::new(Mutex::new(vec)),
      index: Arc::new(Mutex::new(0)),
    }
  }

  /// Create a new stream from a slice
  pub fn from_slice(slice: &[T]) -> Self
  where
    T: Clone,
  {
    Self::new(slice.to_vec())
  }

  /// Get the length of the vector
  pub async fn len(&self) -> usize {
    let vec = self.vec.lock().await;
    vec.len()
  }

  /// Check if the vector is empty
  pub async fn is_empty(&self) -> bool {
    self.len().await == 0
  }
}

impl<T, E> EffectStreamSource<T, E> for VecStreamSource<T>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + Debug + 'static,
{
  type Stream = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn source(&self) -> Self::Stream {
    let stream = EffectStream::<T, E>::new();
    let stream_clone = stream.clone();
    let vec = self.vec.clone();
    let index = self.index.clone();

    Box::pin(async move {
      tokio::spawn(async move {
        let vec = vec.lock().await;
        let mut index = index.lock().await;
        while *index < vec.len() {
          let value = vec[*index].clone();
          *index += 1;
          stream_clone.push(value).await.unwrap();
        }
        stream_clone.close().await.unwrap();
      });

      Ok(stream)
    })
  }
}

#[cfg(test)]
mod tests {
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
  async fn test_vec_stream_basic() {
    let stream = VecStreamSource::new(vec![1, 2, 3]);
    let stream: EffectStream<i32, TestError> = stream.source().await.unwrap();
    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }
    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_vec_stream_empty() {
    let stream: VecStreamSource<i32> = VecStreamSource::new(vec![]);
    let stream: EffectStream<i32, TestError> = stream.source().await.unwrap();
    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }
    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_vec_stream_from_slice() {
    let arr = [1, 2, 3];
    let stream = VecStreamSource::from_slice(&arr);
    let stream: EffectStream<i32, TestError> = stream.source().await.unwrap();
    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }
    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_len_and_is_empty() {
    let stream = VecStreamSource::new(vec![1, 2, 3]);
    assert_eq!(stream.len().await, 3);
    assert!(!stream.is_empty().await);

    let empty_stream: VecStreamSource<i32> = VecStreamSource::new(vec![]);
    assert_eq!(empty_stream.len().await, 0);
    assert!(empty_stream.is_empty().await);
  }
}
