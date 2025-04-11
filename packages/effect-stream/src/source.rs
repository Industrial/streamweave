use crate::stream::EffectStream;
use crate::EffectResult;
use std::future::Future;

/// A trait for types that can produce an EffectStream
pub trait EffectStreamSource<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  /// The type of the stream produced by this source
  type Stream: Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static;

  /// Create a new stream from this source
  fn source(&self) -> Self::Stream;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::EffectError;
  use std::pin::Pin;
  use std::sync::Arc;
  use tokio::sync::Mutex;

  // Test error type
  #[derive(Debug, Clone, PartialEq)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  impl From<EffectError<TestError>> for TestError {
    fn from(e: EffectError<TestError>) -> Self {
      match e {
        EffectError::Custom(e) => e,
        EffectError::Processing(e) => TestError("processing error".to_string()),
        EffectError::Closed => TestError("stream closed".to_string()),
        EffectError::Read(e) => TestError("read error".to_string()),
        EffectError::Write(e) => TestError("write error".to_string()),
      }
    }
  }

  // Test implementation of EffectStreamSource
  struct TestSource<T> {
    values: Vec<T>,
    should_error: bool,
  }

  impl<T> TestSource<T> {
    fn new(values: Vec<T>, should_error: bool) -> Self {
      Self {
        values,
        should_error,
      }
    }
  }

  impl<T, E> EffectStreamSource<T, E> for TestSource<T>
  where
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + From<TestError> + From<EffectError<E>> + 'static,
  {
    type Stream =
      Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

    fn source(&self) -> Self::Stream {
      let stream = EffectStream::<T, E>::new();
      let mut stream_clone = stream.clone();
      let values = self.values.clone();
      let should_error = self.should_error;

      tokio::spawn(async move {
        if should_error {
          let _ = stream_clone
            .set_error(TestError("test error".to_string()).into())
            .await;
        } else {
          for value in values {
            if let Err(e) = stream_clone.push(value).await {
              let _ = stream_clone.set_error(e.into()).await;
              break;
            }
          }
          let _ = stream_clone.close().await;
        }
      });

      Box::pin(async move { Ok(stream) })
    }
  }

  // Test with primitive types
  #[tokio::test]
  async fn test_primitive_types() {
    let source = TestSource::<i32>::new(vec![1, 2, 3], false);
    let stream: EffectStream<i32, TestError> = source.source().await.unwrap();

    assert_eq!(stream.next().await.unwrap(), Some(1));
    assert_eq!(stream.next().await.unwrap(), Some(2));
    assert_eq!(stream.next().await.unwrap(), Some(3));
    assert_eq!(stream.next().await.unwrap(), None);
  }

  // Test with custom types
  #[tokio::test]
  async fn test_custom_types() {
    #[derive(Debug, Clone, PartialEq)]
    struct Point {
      x: i32,
      y: i32,
    }

    let source = TestSource::<Point>::new(vec![Point { x: 1, y: 2 }, Point { x: 3, y: 4 }], false);
    let stream: EffectStream<Point, TestError> = source.source().await.unwrap();

    assert_eq!(stream.next().await.unwrap(), Some(Point { x: 1, y: 2 }));
    assert_eq!(stream.next().await.unwrap(), Some(Point { x: 3, y: 4 }));
    assert_eq!(stream.next().await.unwrap(), None);
  }

  // Test with error handling
  #[tokio::test]
  async fn test_error_handling() {
    let source = TestSource::<i32>::new(vec![], true);
    let stream: EffectStream<i32, TestError> = source.source().await.unwrap();

    assert!(matches!(stream.next().await, Err(EffectError::Custom(_))));
  }

  // Test with concurrent access
  #[tokio::test]
  async fn test_concurrent_access() {
    let source = TestSource::<i32>::new(vec![1, 2, 3], false);
    let stream: EffectStream<i32, TestError> = source.source().await.unwrap();
    let stream_clone = stream.clone();
    let results = Arc::new(Mutex::new(Vec::new()));

    let results_clone = results.clone();
    tokio::spawn(async move {
      while let Ok(Some(value)) = stream_clone.next().await {
        results_clone.lock().await.push(value);
      }
    });

    let mut local_results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      local_results.push(value);
    }

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert_eq!(local_results.len(), 3);
  }

  // Test with empty stream
  #[tokio::test]
  async fn test_empty_stream() {
    let source = TestSource::<i32>::new(vec![], false);
    let stream: EffectStream<i32, TestError> = source.source().await.unwrap();

    assert_eq!(stream.next().await.unwrap(), None);
  }

  // Test with large number of items
  #[tokio::test]
  async fn test_large_stream() {
    let values: Vec<i32> = (0..1000).collect();
    let source = TestSource::<i32>::new(values.clone(), false);
    let stream: EffectStream<i32, TestError> = source.source().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }

    assert_eq!(results, values);
  }
}
