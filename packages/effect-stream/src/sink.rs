use crate::error::EffectResult;
use crate::stream::EffectStream;
use crate::EffectError;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A trait for types that can consume an EffectStream
pub trait EffectStreamSink<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  /// The type of the future returned by this sink
  type Future: Future<Output = EffectResult<(), E>> + Send + 'static;

  /// Consume a stream
  fn sink(&self, stream: EffectStream<T, E>) -> Self::Future;
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::pin::Pin;

  // Test error type
  #[derive(Debug, Clone, PartialEq)]
  struct TestError(String);

  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }

  impl std::error::Error for TestError {}

  impl From<TestError> for EffectError<TestError> {
    fn from(e: TestError) -> Self {
      EffectError::Custom(e)
    }
  }

  // Test implementation of EffectStreamSink
  struct TestSink<F> {
    f: F,
    should_error: bool,
  }

  impl<F> TestSink<F> {
    fn new(f: F, should_error: bool) -> Self {
      Self { f, should_error }
    }
  }

  impl<T, E, F> EffectStreamSink<T, E> for TestSink<F>
  where
    T: Send + Sync + 'static,
    E: Send + Sync + Clone + From<TestError> + 'static,
    F: Fn(T) + Send + Sync + 'static,
  {
    type Future = Pin<Box<dyn Future<Output = EffectResult<(), E>> + Send + 'static>>;

    fn sink(&self, stream: EffectStream<T, E>) -> Self::Future {
      let f = &self.f;
      let should_error = self.should_error;

      Box::pin(async move {
        if should_error {
          Err(TestError("test error".to_string()).into())
        } else {
          while let Ok(Some(value)) = stream.next().await {
            f(value);
          }
          Ok(())
        }
      })
    }
  }

  // Test with primitive types
  #[tokio::test]
  async fn test_primitive_types() {
    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x| {
        results_clone.lock().blocking_lock().push(x);
      },
      false,
    );
    let stream = EffectStream::<i32, TestError>::new();
    sink.sink(stream).await.unwrap();
  }

  // Test with custom types
  #[tokio::test]
  async fn test_custom_types() {
    #[derive(Debug, Clone, PartialEq)]
    struct Point {
      x: i32,
      y: i32,
    }

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |p: Point| {
        results_clone.lock().blocking_lock().push(p);
      },
      false,
    );
    let stream = EffectStream::<Point, TestError>::new();
    sink.sink(stream).await.unwrap();
  }

  // Test with error handling
  #[tokio::test]
  async fn test_error_handling() {
    let sink = TestSink::new(|_| {}, true);
    let stream = EffectStream::<i32, TestError>::new();
    assert!(matches!(
      sink.sink(stream).await,
      Err(EffectError::Custom(_))
    ));
  }

  // Test with concurrent access
  #[tokio::test]
  async fn test_concurrent_access() {
    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x: i32| {
        results_clone.lock().blocking_lock().push(x);
      },
      false,
    );
    let stream = EffectStream::<i32, TestError>::new();
    sink.sink(stream).await.unwrap();
  }

  // Test with empty stream
  #[tokio::test]
  async fn test_empty_stream() {
    let sink = TestSink::new(|_| {}, false);
    let stream = EffectStream::<i32, TestError>::new();
    assert!(sink.sink(stream).await.is_ok());
  }

  // Test with large number of items
  #[tokio::test]
  async fn test_large_stream() {
    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x: i32| {
        results_clone.lock().blocking_lock().push(x);
      },
      false,
    );
    let stream = EffectStream::<i32, TestError>::new();
    sink.sink(stream).await.unwrap();
  }
}
