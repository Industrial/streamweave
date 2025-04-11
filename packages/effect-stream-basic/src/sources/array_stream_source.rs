use effect_stream::{EffectResult, EffectStream, EffectStreamSource};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

/// A stream that produces values from an array
pub struct ArrayStreamSource<T, const N: usize>
where
  T: Send + Sync + Clone + 'static,
{
  array: [T; N],
}

impl<T, const N: usize> ArrayStreamSource<T, N>
where
  T: Send + Sync + Clone + 'static,
{
  /// Create a new ArrayStream from an array
  pub fn new(array: [T; N]) -> Self {
    Self { array }
  }

  /// Create a new ArrayStream from a slice
  pub fn from_slice(slice: &[T]) -> Option<Self>
  where
    T: Copy,
  {
    if slice.len() != N {
      return None;
    }
    let mut arr = std::mem::MaybeUninit::<[T; N]>::uninit();
    let ptr = arr.as_mut_ptr() as *mut T;
    unsafe {
      for (i, &item) in slice.iter().enumerate() {
        ptr.add(i).write(item);
      }
      Some(Self::new(arr.assume_init()))
    }
  }

  /// Get the length of the array
  pub fn len(&self) -> usize {
    N
  }

  /// Check if the array is empty
  pub fn is_empty(&self) -> bool {
    N == 0
  }
}

impl<T, E, const N: usize> EffectStreamSource<T, E> for ArrayStreamSource<T, N>
where
  T: Send + Sync + Clone + 'static,
  E: Send + Sync + Clone + Debug + 'static,
{
  type Stream = Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

  fn source(&self) -> Self::Stream {
    let stream = EffectStream::<T, E>::new();
    let stream_clone = stream.clone();
    let array = self.array.clone();

    Box::pin(async move {
      // Push values in a separate task
      tokio::spawn(async move {
        for value in array {
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
  async fn test_array_stream_basic() {
    let array = [1, 2, 3, 4, 5];
    let stream = ArrayStreamSource::new(array);
    let stream: EffectStream<i32, TestError> = stream.source().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn test_array_stream_empty() {
    let array: [i32; 0] = [];
    let stream = ArrayStreamSource::new(array);
    let stream: EffectStream<i32, TestError> = stream.source().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }

    assert_eq!(results, Vec::<i32>::new());
  }

  #[tokio::test]
  async fn test_array_stream_strings() {
    let array = ["a", "b", "c"];
    let stream = ArrayStreamSource::new(array);
    let stream: EffectStream<&str, TestError> = stream.source().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec!["a", "b", "c"]);
  }

  #[tokio::test]
  async fn test_array_stream_from_slice_success() {
    let slice = &[1, 2, 3];
    let stream = ArrayStreamSource::<_, 3>::from_slice(slice).unwrap();
    let stream: EffectStream<i32, TestError> = stream.source().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }

    assert_eq!(results, vec![1, 2, 3]);
  }

  #[tokio::test]
  async fn test_array_stream_from_slice_failure() {
    let slice = &[1, 2, 3];
    assert!(ArrayStreamSource::<_, 4>::from_slice(slice).is_none());
  }

  #[tokio::test]
  async fn test_len_and_is_empty() {
    let stream: ArrayStreamSource<i32, 3> = ArrayStreamSource::new([1, 2, 3]);
    assert_eq!(stream.len(), 3);
    assert!(!stream.is_empty());

    let empty_stream: ArrayStreamSource<i32, 0> = ArrayStreamSource::new([]);
    assert_eq!(empty_stream.len(), 0);
    assert!(empty_stream.is_empty());
  }
}
