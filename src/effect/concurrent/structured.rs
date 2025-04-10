//! Structured concurrency implementation.
//!
//! This module provides functions for structured concurrent execution of effects,
//! including parallel and sequential execution.

use std::error::Error as StdError;
use std::future::Future;

use effect_core::effect::Effect;
use effect_core::monad::Monad;

/// Executes a sequence of effects in parallel.
pub fn parallel<T, E, I>(effects: I) -> Effect<Vec<T>, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  I: IntoIterator<Item = Effect<T, E>>,
  I::IntoIter: Send + 'static,
{
  Effect::new(async move {
    let futures: Vec<_> = effects.into_iter().map(|effect| effect.run()).collect();
    let results = futures::future::join_all(futures).await;
    let mut output = Vec::with_capacity(results.len());

    for result in results {
      output.push(result?);
    }

    Ok(output)
  })
}

/// Executes a sequence of effects in order.
pub fn sequence<T, E, I>(effects: I) -> Effect<Vec<T>, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  I: IntoIterator<Item = Effect<T, E>>,
  I::IntoIter: Send + 'static,
{
  Effect::new(async move {
    let mut output = Vec::new();

    for effect in effects {
      output.push(effect.run().await?);
    }

    Ok(output)
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};
  use std::time::Duration;
  use tokio::time::sleep;

  // Basic parallel execution tests
  #[tokio::test]
  async fn test_parallel_basic() {
    let effects = vec![
      Effect::new(async { Ok::<_, IoError>(1) }),
      Effect::new(async { Ok(2) }),
      Effect::new(async { Ok(3) }),
    ];

    let result = parallel(effects).run().await.unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
  }

  // Test parallel execution with delays
  #[tokio::test]
  async fn test_parallel_with_delays() {
    let effects = vec![
      Effect::new(async {
        sleep(Duration::from_millis(50)).await;
        Ok::<_, IoError>(1)
      }),
      Effect::new(async {
        sleep(Duration::from_millis(10)).await;
        Ok(2)
      }),
      Effect::new(async {
        sleep(Duration::from_millis(30)).await;
        Ok(3)
      }),
    ];

    let start = std::time::Instant::now();
    let result = parallel(effects).run().await.unwrap();
    let duration = start.elapsed();

    assert_eq!(result.len(), 3);
    assert!(result.contains(&1));
    assert!(result.contains(&2));
    assert!(result.contains(&3));
    // Should complete in roughly 50ms, not 90ms
    assert!(duration < Duration::from_millis(70));
  }

  // Basic sequence execution tests
  #[tokio::test]
  async fn test_sequence_basic() {
    let effects = vec![
      Effect::new(async { Ok::<_, IoError>(1) }),
      Effect::new(async { Ok(2) }),
      Effect::new(async { Ok(3) }),
    ];

    let result = sequence(effects).run().await.unwrap();
    assert_eq!(result, vec![1, 2, 3]);
  }

  // Test sequence execution with delays
  #[tokio::test]
  async fn test_sequence_with_delays() {
    let effects = vec![
      Effect::new(async {
        sleep(Duration::from_millis(30)).await;
        Ok::<_, IoError>(1)
      }),
      Effect::new(async {
        sleep(Duration::from_millis(20)).await;
        Ok(2)
      }),
      Effect::new(async {
        sleep(Duration::from_millis(10)).await;
        Ok(3)
      }),
    ];

    let start = std::time::Instant::now();
    let result = sequence(effects).run().await.unwrap();
    let duration = start.elapsed();

    assert_eq!(result, vec![1, 2, 3]);
    // Should take at least 60ms (sum of all delays)
    assert!(duration >= Duration::from_millis(55));
  }

  // Error handling tests
  #[tokio::test]
  async fn test_parallel_error() {
    let effects = vec![
      Effect::pure(1),
      Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "test error")) }),
      Effect::pure(3),
    ];

    let result = parallel(effects).run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "test error");
  }

  #[tokio::test]
  async fn test_sequence_error() {
    let effects = vec![
      Effect::pure(1),
      Effect::new(async { Err::<i32, _>(IoError::new(ErrorKind::Other, "test error")) }),
      Effect::pure(3),
    ];

    let result = sequence(effects).run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "test error");
  }

  // Empty input tests
  #[tokio::test]
  async fn test_parallel_empty() {
    let effects: Vec<Effect<i32, IoError>> = vec![];
    let result = parallel(effects).run().await.unwrap();
    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_sequence_empty() {
    let effects: Vec<Effect<i32, IoError>> = vec![];
    let result = sequence(effects).run().await.unwrap();
    assert!(result.is_empty());
  }

  // Single effect tests
  #[tokio::test]
  async fn test_parallel_single() {
    let effects = vec![Effect::pure::<_, IoError>(42)];
    let result = parallel(effects).run().await.unwrap();
    assert_eq!(result, vec![42]);
  }

  #[tokio::test]
  async fn test_sequence_single() {
    let effects = vec![Effect::pure::<_, IoError>(42)];
    let result = sequence(effects).run().await.unwrap();
    assert_eq!(result, vec![42]);
  }

  // Mixed effect types tests
  #[tokio::test]
  async fn test_parallel_mixed_types() {
    let effects = vec![
      Effect::pure::<_, IoError>("hello".to_string()),
      Effect::new(async { Ok("world".to_string()) }),
      Effect::new(async {
        sleep(Duration::from_millis(10)).await;
        Ok("!".to_string())
      }),
    ];

    let result = parallel(effects).run().await.unwrap();
    assert_eq!(result.len(), 3);
    assert!(result.contains(&"hello".to_string()));
    assert!(result.contains(&"world".to_string()));
    assert!(result.contains(&"!".to_string()));
  }

  // Error propagation tests
  #[tokio::test]
  async fn test_parallel_error_propagation() {
    let effects = vec![
      Effect::new(async { Ok::<i32, IoError>(1) }),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "first error")) }),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "second error")) }),
    ];

    let result = parallel(effects).run().await;
    assert!(result.is_err());
    // First error should be propagated
    assert!(result.unwrap_err().to_string().contains("first error"));
  }

  #[tokio::test]
  async fn test_sequence_error_propagation() {
    let effects = vec![
      Effect::new(async { Ok::<i32, IoError>(1) }),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "first error")) }),
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "second error")) }),
    ];

    let result = sequence(effects).run().await;
    assert!(result.is_err());
    // First error should be propagated
    assert!(result.unwrap_err().to_string().contains("first error"));
  }

  // Large input tests
  #[tokio::test]
  async fn test_parallel_large_input() {
    let effects: Vec<Effect<i32, IoError>> = (0..1000).map(|i| Effect::pure(i)).collect();

    let result = parallel(effects).run().await.unwrap();
    assert_eq!(result.len(), 1000);
    for i in 0..1000 {
      assert!(result.contains(&i));
    }
  }

  #[tokio::test]
  async fn test_sequence_large_input() {
    let effects: Vec<Effect<i32, IoError>> = (0..1000).map(|i| Effect::pure(i)).collect();

    let result = sequence(effects).run().await.unwrap();
    assert_eq!(result.len(), 1000);
    for (i, &val) in result.iter().enumerate() {
      assert_eq!(val, i as i32);
    }
  }
}
