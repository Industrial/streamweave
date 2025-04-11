use crate::error::EffectResult;
use crate::stream::EffectStream;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A trait for types that can consume an EffectStream
pub trait EffectStreamSink<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  /// The type of the result
  type Future: Future<Output = EffectResult<(), E>> + Send + 'static;

  /// Consume the stream
  fn consume(&self, stream: EffectStream<T, E>) -> Self::Future;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::EffectError;
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
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + From<TestError> + From<EffectError<E>> + 'static,
    F: Fn(T) + Send + Sync + Clone + 'static,
    EffectError<E>: From<TestError>,
  {
    type Future = Pin<Box<dyn Future<Output = EffectResult<(), E>> + Send + 'static>>;

    fn consume(&self, stream: EffectStream<T, E>) -> Self::Future {
      let mut stream_clone = stream.clone();
      let f = self.f.clone();
      let should_error = self.should_error;

      Box::pin(async move {
        if should_error {
          Err(TestError("test error".to_string()).into())
        } else {
          while let Ok(Some(value)) = stream_clone.next().await {
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
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x| {
        let results = results_clone.clone();
        tokio::spawn(async move {
          results.lock().await.push(x);
        });
      },
      false,
    );

    sink.consume(stream).await.unwrap();

    // Give the spawned tasks time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert_eq!(final_results[0], 1);
    assert_eq!(final_results[1], 2);
    assert_eq!(final_results[2], 3);
  }

  // Test with custom types
  #[tokio::test]
  async fn test_custom_types() {
    #[derive(Debug, Clone, PartialEq)]
    struct Point {
      x: i32,
      y: i32,
    }

    let stream = EffectStream::<Point, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(Point { x: 1, y: 2 }).await.unwrap();
      stream_clone.push(Point { x: 3, y: 4 }).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |p: Point| {
        let results = results_clone.clone();
        tokio::spawn(async move {
          results.lock().await.push(p);
        });
      },
      false,
    );

    sink.consume(stream).await.unwrap();

    // Give the spawned tasks time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 2);
    assert_eq!(final_results[0], Point { x: 1, y: 2 });
    assert_eq!(final_results[1], Point { x: 3, y: 4 });
  }

  // Test with error handling
  #[tokio::test]
  async fn test_error_handling() {
    let stream = EffectStream::<i32, TestError>::new();
    let sink = TestSink::<fn(i32)>::new(|_| {}, true);

    assert!(matches!(
      sink.consume(stream).await,
      Err(EffectError::Custom(_))
    ));
  }

  // Test with concurrent access
  #[tokio::test]
  async fn test_concurrent_access() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    tokio::spawn(async move {
      stream_clone.push(1).await.unwrap();
      stream_clone.push(2).await.unwrap();
      stream_clone.push(3).await.unwrap();
      stream_clone.close().await.unwrap();
    });

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x| {
        let results = results_clone.clone();
        tokio::spawn(async move {
          results.lock().await.push(x);
        });
      },
      false,
    );

    sink.consume(stream).await.unwrap();

    // Give the spawned tasks time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert_eq!(final_results[0], 1);
    assert_eq!(final_results[1], 2);
    assert_eq!(final_results[2], 3);
  }

  // Test with empty stream
  #[tokio::test]
  async fn test_empty_stream() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();

    // Close the stream immediately
    tokio::spawn(async move {
      stream_clone.close().await.unwrap();
    });

    let results = Arc::new(Mutex::new(Vec::new()));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x| {
        let results = results_clone.clone();
        tokio::spawn(async move {
          results.lock().await.push(x);
        });
      },
      false,
    );

    sink.consume(stream).await.unwrap();

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 0);
  }

  // Test with large number of items
  #[tokio::test]
  async fn test_large_stream() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();
    let values: Vec<i32> = (0..1000).collect();

    tokio::spawn(async move {
      for value in values {
        stream_clone.push(value).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let results = Arc::new(Mutex::new(Vec::with_capacity(1000)));
    let results_clone = results.clone();
    let sink = TestSink::new(
      move |x| {
        let results = results_clone.clone();
        tokio::spawn(async move {
          results.lock().await.push(x);
        });
      },
      false,
    );

    sink.consume(stream).await.unwrap();

    // Give the spawned tasks time to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 1000);
    for i in 0..1000 {
      assert_eq!(final_results[i], i as i32);
    }
  }
}
