//! Error recovery strategies for the effect system.
//!
//! This module provides traits and implementations for error recovery in the
//! effect system. It includes the `Recover` and `RecoverWith` traits, which
//! define how to recover from errors in effect computations.

use effect_core::effect::Effect;
use std::error::Error as StdError;

/// A trait for recovering from errors in effect computations.
pub trait Recover<E> {
  /// The type of the value produced by the effect.
  type Output;

  /// Recovers from an error by providing a default value.
  fn recover<F>(self, f: F) -> Effect<Self::Output, E>
  where
    F: FnOnce(E) -> Self::Output + Send + Sync + 'static;

  /// Recovers from an error by providing a default effect.
  fn recover_with<F>(self, f: F) -> Effect<Self::Output, E>
  where
    F: FnOnce(E) -> Effect<Self::Output, E> + Send + Sync + 'static;
}

/// A trait for recovering from errors with a specific error type.
pub trait RecoverWith<E, F> {
  /// The type of the value produced by the effect.
  type Output;

  /// Recovers from an error by providing a default effect.
  fn recover_with(self, f: F) -> Effect<Self::Output, E>;
}

impl<T, E> Recover<E> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  type Output = T;

  fn recover<F>(self, f: F) -> Effect<T, E>
  where
    F: FnOnce(E) -> T + Send + Sync + 'static,
  {
    Effect::new(async move {
      match self.run().await {
        Ok(value) => Ok(value),
        Err(error) => Ok(f(error)),
      }
    })
  }

  fn recover_with<F>(self, f: F) -> Effect<T, E>
  where
    F: FnOnce(E) -> Effect<T, E> + Send + Sync + 'static,
  {
    Effect::new(async move {
      match self.run().await {
        Ok(value) => Ok(value),
        Err(error) => f(error).run().await,
      }
    })
  }
}

impl<T, E, F> RecoverWith<E, F> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  F: FnOnce(E) -> Effect<T, E> + Send + Sync + 'static,
{
  type Output = T;

  fn recover_with(self, f: F) -> Effect<T, E> {
    self.recover_with(f)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{self, Error as IoError, ErrorKind};
  use tokio::test;

  // Basic recovery tests
  #[tokio::test]
  async fn test_recover_success() {
    let effect = Effect::<i32, IoError>::new(async move { Ok(42) });
    let recovered = effect.recover(|_| 0);
    assert_eq!(recovered.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_recover_error() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover(|_| 42);
    assert_eq!(recovered.run().await, Ok(42));
  }

  // Recover with tests
  #[tokio::test]
  async fn test_recover_with_success() {
    let effect = Effect::<i32, IoError>::new(async move { Ok(42) });
    let recovered = effect.recover_with(|_| Effect::new(async { Ok(0) }));
    assert_eq!(recovered.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_recover_with_error() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover_with(|_| Effect::new(async { Ok(42) }));
    assert_eq!(recovered.run().await, Ok(42));
  }

  // Complex recovery tests
  #[tokio::test]
  async fn test_recover_with_nested_effects() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover_with(|_| {
      Effect::new(async move {
        let inner = Effect::new(async { Err(IoError::new(ErrorKind::Other, "inner error")) });
        inner.recover(|_| 42).run().await
      })
    });
    assert_eq!(recovered.run().await, Ok(42));
  }

  // Error transformation tests
  #[tokio::test]
  async fn test_recover_with_error_transformation() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover_with(|error| {
      Effect::new(async move { Err(IoError::new(ErrorKind::Other, "new error")) })
    });
    let result = recovered.run().await;
    assert!(result.is_err());
  }

  // Multiple recovery chain tests
  #[tokio::test]
  async fn test_multiple_recovery_chain() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect
      .recover_with(|_| Effect::new(async { Err(IoError::new(ErrorKind::Other, "second error")) }))
      .recover_with(|_| Effect::new(async { Err(IoError::new(ErrorKind::Other, "third error")) }))
      .recover(|_| 42);
    assert_eq!(recovered.run().await, Ok(42));
  }

  // Different value type tests
  #[tokio::test]
  async fn test_recover_different_types() {
    let effect =
      Effect::<String, IoError>::new(
        async move { Err(IoError::new(ErrorKind::Other, "test error")) },
      );
    let recovered = effect.recover(|_| "recovered".to_string());
    assert_eq!(recovered.run().await, Ok("recovered".to_string()));

    let effect =
      Effect::<Vec<i32>, IoError>::new(
        async move { Err(IoError::new(ErrorKind::Other, "test error")) },
      );
    let recovered = effect.recover(|_| vec![1, 2, 3]);
    assert_eq!(recovered.run().await, Ok(vec![1, 2, 3]));
  }

  // Async recovery tests
  #[tokio::test]
  async fn test_async_recovery() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover_with(|_| {
      Effect::new(async {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        Ok(42)
      })
    });
    assert_eq!(recovered.run().await, Ok(42));
  }

  // Error inspection tests
  #[tokio::test]
  async fn test_error_inspection() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover(|error| {
      assert_eq!(error.kind(), ErrorKind::Other);
      42
    });
    assert_eq!(recovered.run().await, Ok(42));
  }

  // RecoverWith trait tests
  #[tokio::test]
  async fn test_recover_with_trait() {
    let effect =
      Effect::<i32, IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover_with(|_| Effect::new(async { Ok(42) }));
    assert_eq!(recovered.run().await, Ok(42));
  }

  // Zero-sized type tests
  #[tokio::test]
  async fn test_zero_sized_types() {
    let effect =
      Effect::<(), IoError>::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) });
    let recovered = effect.recover(|_| ());
    assert_eq!(recovered.run().await, Ok(()));

    let value = 42;
    let effect =
      Effect::<&i32, IoError>::new(
        async move { Err(IoError::new(ErrorKind::Other, "test error")) },
      );
    let recovered = effect.recover(|_| &value);
    assert_eq!(recovered.run().await, Ok(&42));
  }
}
