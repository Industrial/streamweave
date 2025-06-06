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
        EffectError::Processing(_) => TestError("processing error".to_string()),
        EffectError::Closed => TestError("stream closed".to_string()),
        EffectError::Read(_) => TestError("read error".to_string()),
        EffectError::Write(_) => TestError("write error".to_string()),
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
    E: Send + Sync + Clone + From<TestError> + From<EffectError<E>> + std::fmt::Debug + 'static,
    EffectError<E>: From<TestError>,
  {
    type Stream =
      Pin<Box<dyn Future<Output = EffectResult<EffectStream<T, E>, E>> + Send + 'static>>;

    fn source(&self) -> Self::Stream {
      let stream = EffectStream::<T, E>::new();
      let stream_clone = stream.clone();
      let values = self.values.clone();
      let should_error = self.should_error;

      Box::pin(async move {
        if should_error {
          Err(TestError("test error".to_string()).into())
        } else {
          // Push values in a separate task
          tokio::spawn(async move {
            for value in values {
              stream_clone.push(value).await.unwrap();
            }
            stream_clone.close().await.unwrap();
          });
          Ok(stream)
        }
      })
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
    assert!(matches!(source.source().await, Err(EffectError::Custom(_))));
  }

  // Test with concurrent access
  #[tokio::test]
  async fn test_concurrent_access() {
    let source = TestSource::<i32>::new(vec![1, 2, 3], false);
    let stream: EffectStream<i32, TestError> = source.source().await.unwrap();
    let stream_clone = stream.clone();

    // Create a shared results vector
    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();

    // Create two concurrent consumers
    let consumer1 = tokio::spawn({
      let results = results.clone();
      async move {
        while let Ok(Some(value)) = stream.next().await {
          results.lock().await.push(value);
        }
      }
    });

    let consumer2 = tokio::spawn(async move {
      while let Ok(Some(value)) = stream_clone.next().await {
        results_clone.lock().await.push(value);
      }
    });

    // Wait for both consumers to finish
    consumer1.await.unwrap();
    consumer2.await.unwrap();

    // Get the final results
    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert!(final_results.contains(&1));
    assert!(final_results.contains(&2));
    assert!(final_results.contains(&3));
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

    let mut results = Vec::with_capacity(1000);
    while let Ok(Some(value)) = stream.next().await {
      results.push(value);
    }

    assert_eq!(results.len(), 1000);
    for (i, &value) in results.iter().enumerate() {
      assert_eq!(value, i as i32);
    }
  }
}
