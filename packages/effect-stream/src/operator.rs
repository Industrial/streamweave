use crate::error::EffectResult;
use crate::stream::EffectStream;
use std::future::Future;

/// A trait for types that can transform an EffectStream
pub trait EffectStreamOperator<T, U, E>
where
  T: Send + Sync + 'static,
  U: Send + Sync + 'static,
  E: Send + Sync + Clone + 'static,
{
  /// The type of the transformed stream
  type Stream: Future<Output = EffectResult<EffectStream<U, E>, E>> + Send + 'static;

  /// Apply the operator to a stream
  fn apply(&self, stream: EffectStream<T, E>) -> Self::Stream;
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

  impl From<TestError> for EffectError<TestError> {
    fn from(e: TestError) -> Self {
      EffectError::Custom(e)
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

  impl<T, U, E, F> EffectStreamOperator<T, U, E> for TestOperator<F>
  where
    T: Send + Sync + Clone + 'static,
    U: Send + Sync + Clone + 'static,
    E: Send + Sync + Clone + From<TestError> + 'static,
    F: Fn(T) -> U + Send + Sync + 'static,
  {
    type Stream =
      Pin<Box<dyn Future<Output = EffectResult<EffectStream<U, E>, E>> + Send + 'static>>;

    fn apply(&self, stream: EffectStream<T, E>) -> Self::Stream {
      let new_stream = EffectStream::<U, E>::new();
      let mut new_stream_clone = new_stream.clone();
      let mut stream_clone = stream.clone();
      let f = &self.f;
      let should_error = self.should_error;

      tokio::spawn(async move {
        if should_error {
          let _ = new_stream_clone
            .set_error(TestError("test error".to_string()).into())
            .await;
        } else {
          while let Ok(Some(value)) = stream_clone.next().await {
            if let Err(e) = new_stream_clone.push(f(value)).await {
              let _ = new_stream_clone.set_error(e.into()).await;
              break;
            }
          }
          let _ = new_stream_clone.close().await;
        }
      });

      Box::pin(async move { Ok(new_stream) })
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
    let mapped_stream = operator.apply(stream).await.unwrap();

    assert_eq!(mapped_stream.next().await.unwrap(), Some(2));
    assert_eq!(mapped_stream.next().await.unwrap(), Some(4));
    assert_eq!(mapped_stream.next().await.unwrap(), Some(6));
    assert_eq!(mapped_stream.next().await.unwrap(), None);
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
    let mapped_stream = operator.apply(stream).await.unwrap();

    assert_eq!(
      mapped_stream.next().await.unwrap(),
      Some(Point { x: 2, y: 4 })
    );
    assert_eq!(
      mapped_stream.next().await.unwrap(),
      Some(Point { x: 6, y: 8 })
    );
    assert_eq!(mapped_stream.next().await.unwrap(), None);
  }

  // Test with error handling
  #[tokio::test]
  async fn test_error_handling() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = TestOperator::<fn(i32) -> i32>::new(|x| x, true);
    let mapped_stream = operator.apply(stream).await.unwrap();

    assert!(matches!(
      mapped_stream.next().await,
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
    let mapped_stream = operator.apply(stream).await.unwrap();
    let mapped_stream_clone = mapped_stream.clone();
    let results = Arc::new(Mutex::new(Vec::new()));

    let results_clone = results.clone();
    tokio::spawn(async move {
      while let Ok(Some(value)) = mapped_stream_clone.next().await {
        results_clone.lock().await.push(value);
      }
    });

    let mut local_results = Vec::new();
    while let Ok(Some(value)) = mapped_stream.next().await {
      local_results.push(value);
    }

    let final_results = results.lock().await;
    assert_eq!(final_results.len(), 3);
    assert_eq!(local_results.len(), 3);
  }

  // Test with empty stream
  #[tokio::test]
  async fn test_empty_stream() {
    let stream = EffectStream::<i32, TestError>::new();
    let operator = TestOperator::new(|x: i32| x * 2, false);
    let mapped_stream = operator.apply(stream).await.unwrap();

    assert_eq!(mapped_stream.next().await.unwrap(), None);
  }

  // Test with large number of items
  #[tokio::test]
  async fn test_large_stream() {
    let stream = EffectStream::<i32, TestError>::new();
    let mut stream_clone = stream.clone();
    let values: Vec<i32> = (0..1000).collect();

    tokio::spawn(async move {
      for value in values.clone() {
        stream_clone.push(value).await.unwrap();
      }
      stream_clone.close().await.unwrap();
    });

    let operator = TestOperator::new(|x| x * 2, false);
    let mapped_stream = operator.apply(stream).await.unwrap();

    let mut results = Vec::new();
    while let Ok(Some(value)) = mapped_stream.next().await {
      results.push(value);
    }

    assert_eq!(results, values.iter().map(|x| x * 2).collect::<Vec<_>>());
  }
}
