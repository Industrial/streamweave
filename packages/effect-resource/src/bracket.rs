//! Bracket pattern implementation for resource management.
//!
//! This module provides the `Bracket` trait and implementations for the bracket
//! pattern, which ensures that resources are properly acquired and released,
//! even in the presence of errors.

use std::error::Error as StdError;

use effect_core::effect::Effect;

/// A trait for the bracket pattern in resource management.
pub trait Bracket {
  /// The type of the resource.
  type Resource: Clone + Send + Sync + 'static;
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
      let result = use_resource(resource.clone()).run().await;
      release(resource).run().await?;
      result
    })
  }
}

impl<T, E> Bracket for Effect<T, E>
where
  T: Clone + Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  type Resource = T;
  type Error = E;
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::Arc;

  #[tokio::test]
  async fn test_bracket() {
    let acquired = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));
    let acquired_clone = Arc::clone(&acquired);
    let released_clone = Arc::clone(&released);

    let mut effect = Effect::<(), IoError>::bracket(
      move || {
        acquired_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      },
      |_| Effect::pure(()),
      move |_| {
        released_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      },
    );

    effect.run().await.unwrap();
    assert!(acquired.load(Ordering::SeqCst));
    assert!(released.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn test_bracket_with_error() {
    let acquired = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));
    let acquired_clone = Arc::clone(&acquired);
    let released_clone = Arc::clone(&released);

    let mut effect = Effect::<(), IoError>::bracket(
      move || {
        acquired_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      },
      |_| Effect::new(async move { Err(IoError::new(ErrorKind::Other, "test error")) }),
      move |_| {
        released_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      },
    );

    assert!(effect.run().await.is_err());
    assert!(acquired.load(Ordering::SeqCst));
    assert!(released.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn test_bracket_acquire_error() {
    let released = Arc::new(AtomicBool::new(false));
    let released_clone = Arc::clone(&released);

    let mut effect = Effect::<(), IoError>::bracket(
      || Effect::new(async { Err(IoError::new(ErrorKind::Other, "acquire error")) }),
      |_| Effect::pure(()),
      move |_| {
        released_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      },
    );

    let result = effect.run().await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "acquire error");
    assert!(
      !released.load(Ordering::SeqCst),
      "Release should not be called if acquire fails"
    );
  }

  #[tokio::test]
  async fn test_bracket_release_error() {
    let acquired = Arc::new(AtomicBool::new(false));
    let acquired_clone = Arc::clone(&acquired);

    let mut effect = Effect::<(), IoError>::bracket(
      move || {
        acquired_clone.store(true, Ordering::SeqCst);
        Effect::pure(())
      },
      |_| Effect::pure(()),
      |_| Effect::new(async { Err(IoError::new(ErrorKind::Other, "release error")) }),
    );

    let result = effect.run().await;
    assert!(result.is_err());
    assert!(acquired.load(Ordering::SeqCst));
    assert_eq!(result.unwrap_err().to_string(), "release error");
  }
}
