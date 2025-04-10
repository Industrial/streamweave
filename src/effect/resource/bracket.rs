//! Bracket pattern implementation for resource management.
//!
//! This module provides the `Bracket` trait and implementations for the bracket
//! pattern, which ensures that resources are properly acquired and released,
//! even in the presence of errors.

use std::error::Error as StdError;
use std::future::Future;

use effect_core::effect::Effect;

/// A trait for the bracket pattern in resource management.
pub trait Bracket {
  /// The type of the resource.
  type Resource;
  /// The type of the error that can occur.
  type Error: StdError + Send + Sync + 'static;

  /// Acquires a resource and returns an effect that will release it.
  fn bracket<A, U, F>(acquire: A, use_resource: U, release: F) -> Effect<(), Self::Error>
  where
    A: FnOnce() -> Effect<Self::Resource, Self::Error> + Send + Sync + 'static,
    U: FnOnce(Self::Resource) -> Effect<(), Self::Error> + Send + Sync + 'static,
    F: FnOnce(Self::Resource) -> Effect<(), Self::Error> + Send + Sync + 'static,
  {
    Effect::new(async move {
      let resource = acquire().run().await?;
      let result = use_resource(resource).run().await;
      release(resource).run().await?;
      result
    })
  }
}

impl<T, E> Bracket for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  type Resource = T;
  type Error = E;
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};

  #[tokio::test]
  async fn test_bracket() {
    let mut acquired = false;
    let mut released = false;

    let effect = Effect::<(), IoError>::bracket(
      || {
        acquired = true;
        Effect::pure(())
      },
      |_| Effect::pure(()),
      |_| {
        released = true;
        Effect::pure(())
      },
    );

    effect.run().await.unwrap();
    assert!(acquired);
    assert!(released);
  }

  #[tokio::test]
  async fn test_bracket_with_error() {
    let mut acquired = false;
    let mut released = false;

    let effect = Effect::<(), IoError>::bracket(
      || {
        acquired = true;
        Effect::pure(())
      },
      |_| Effect::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) }),
      |_| {
        released = true;
        Effect::pure(())
      },
    );

    assert!(effect.run().await.is_err());
    assert!(acquired);
    assert!(released);
  }

  #[tokio::test]
  async fn test_bracket_acquire_error() {
    let mut released = false;

    let effect = Effect::<(), IoError>::bracket(
      || Effect::new(async { Err(IoError::new(ErrorKind::Other, "acquire error")) }),
      |_| Effect::pure(()),
      |_| {
        released = true;
        Effect::pure(())
      },
    );

    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "acquire error");
    assert!(!released, "Release should not be called if acquire fails");
  }

  #[tokio::test]
  async fn test_bracket_release_error() {
    let mut acquired = false;

    let effect = Effect::<(), IoError>::bracket(
      || {
        acquired = true;
        Effect::pure(())
      },
      |_| Effect::pure(()),
      |_| Effect::new(async { Err(IoError::new(ErrorKind::Other, "release error")) }),
    );

    let result = effect.run().await;
    assert!(result.is_err());
    assert!(acquired);
    assert_eq!(result.unwrap_err().to_string(), "release error");
  }
}
