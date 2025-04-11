use crate::error::EffectResult;
use crate::stream::EffectStream;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A trait for types that can transform an EffectStream
pub trait EffectStreamOperator<T, E, B>
where
  T: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
  B: Send + Sync + 'static,
{
  /// The type of the result
  type Future: Future<Output = EffectResult<EffectStream<B, E>, E>> + Send + 'static;

  /// Transform the stream
  fn transform(&self, stream: EffectStream<T, E>) -> Self::Future;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::error::EffectError;
  use std::fmt::Debug;
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

  // Test implementation of EffectStreamOperator
  struct TestOperator<F> {
    f: F,
    should_error: bool,
  }

  impl<F> TestOperator<F> {
    fn new(f: F, should_error: bool) -> Self {
      Self { f, should_error }
    }
  }

  impl<T, E, B, F> EffectStreamOperator<T, E, B> for TestOperator<F>
  where
    T: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + From<TestError> + From<EffectError<E>> + Debug + 'static,
    B: Send + Sync + 'static,
    F: Fn(T) -> B + Send + Sync + Clone + 'static,
    EffectError<E>: From<TestError>,
  {
    type Future =
      Pin<Box<dyn Future<Output = EffectResult<EffectStream<B, E>, E>> + Send + 'static>>;

    fn transform(&self, stream: EffectStream<T, E>) -> Self::Future {
      let mut stream_clone = stream.clone();
      let f = self.f.clone();
      let should_error = self.should_error;

      Box::pin(async move {
        if should_error {
          Err(TestError("test error".to_string()).into())
        } else {
          let new_stream = EffectStream::<B, E>::new();
          let mut new_stream_clone = new_stream.clone();

          tokio::spawn(async move {
            while let Ok(Some(value)) = stream_clone.next().await {
              new_stream_clone.push(f(value)).await.unwrap();
            }
            new_stream_clone.close().await.unwrap();
          });

          Ok(new_stream)
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

    let operator = TestOperator::new(|x| x * 2, false);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results.len(), 3);
    assert_eq!(results[0], 2);
    assert_eq!(results[1], 4);
    assert_eq!(results[2], 6);
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

    let operator = TestOperator::new(
      |p: Point| Point {
        x: p.x * 2,
        y: p.y * 2,
      },
      false,
    );
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results.len(), 2);
    assert_eq!(results[0], Point { x: 2, y: 4 });
    assert_eq!(results[1], Point { x: 6, y: 8 });
  }

  // Test with error handling
  #[tokio::test]
  async fn test_error_handling() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = TestOperator::<fn(i32) -> i32>::new(|x| x * 2, true);

    assert!(matches!(
      operator.transform(stream).await,
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

    let operator = TestOperator::new(|x| x * 2, false);
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
    assert_eq!(final_results.len(), 3); // Each value is consumed exactly once
    assert!(final_results.contains(&2));
    assert!(final_results.contains(&4));
    assert!(final_results.contains(&6));
  }

  // Test with empty stream
  #[tokio::test]
  async fn test_empty_stream() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = TestOperator::new(|x| x * 2, false);
    let new_stream = operator.transform(stream).await.unwrap();
    new_stream.close().await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results.len(), 0);
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

    let operator = TestOperator::new(|x| x * 2, false);
    let new_stream = operator.transform(stream).await.unwrap();

    let mut results = Vec::with_capacity(1000);
    while let Ok(Some(value)) = new_stream.next().await {
      results.push(value);
    }

    assert_eq!(results.len(), 1000);
    for i in 0..1000 {
      assert_eq!(results[i], (i as i32) * 2);
    }
  }
}
