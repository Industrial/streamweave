//! Utility types for the effect system.
//!
//! This module provides utility types that are commonly used in the effect
//! system.

use std::error::Error as StdError;
use std::future::Future;
use std::pin::Pin;

use crate::effect::core::effect::Effect;

/// A type that represents a deferred effect.
pub struct Deferred<T, E: StdError + Send + Sync + 'static, F> {
  f: F,
  _phantom: std::marker::PhantomData<(T, E)>,
}

impl<T, E, F> Deferred<T, E, F>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
  F: FnOnce() -> Effect<T, E> + Send + Sync + 'static,
{
  /// Creates a new deferred effect.
  pub fn new(f: F) -> Self {
    Self {
      f,
      _phantom: std::marker::PhantomData,
    }
  }

  /// Forces the evaluation of the deferred effect.
  pub fn force(self) -> Effect<T, E> {
    Effect::new(async move { (self.f)().run().await })
  }
}

/// A type that represents a memoized effect.
pub struct Memoized<T, E: StdError + Send + Sync + 'static> {
  inner: std::sync::Arc<tokio::sync::OnceCell<Result<T, E>>>,
  effect: Effect<T, E>,
}

impl<T, E> Memoized<T, E>
where
  T: Clone + Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  /// Creates a new memoized effect.
  pub fn new(effect: Effect<T, E>) -> Self {
    Self {
      inner: std::sync::Arc::new(tokio::sync::OnceCell::new()),
      effect,
    }
  }

  /// Gets the result of the memoized effect.
  pub fn get(&self) -> Effect<T, E> {
    let inner = self.inner.clone();
    let effect = self.effect.clone();

    Effect::new(async move {
      inner
        .get_or_init(|| async { effect.run().await })
        .await
        .clone()
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{Error as IoError, ErrorKind};
  use std::sync::atomic::{AtomicUsize, Ordering};

  // Deferred tests
  #[tokio::test]
  async fn test_deferred_basic() {
    let mut called = false;
    let deferred = Deferred::new(|| {
      called = true;
      Effect::new(async { Ok(42) })
    });
    assert!(!called);
    let result = deferred.force().run().await;
    assert!(called);
    assert_eq!(result, Ok(42));
  }

  #[tokio::test]
  async fn test_deferred_error() {
    let deferred =
      Deferred::new(|| Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) }));
    let result = deferred.force().run().await;
    assert!(result.is_err());
    if let Err(e) = result {
      assert_eq!(e.to_string(), "test error");
    }
  }

  #[tokio::test]
  async fn test_deferred_different_types() {
    let deferred = Deferred::new(|| Effect::new(async { Ok("hello".to_string()) }));
    let result = deferred.force().run().await;
    assert_eq!(result, Ok("hello".to_string()));

    let deferred = Deferred::new(|| Effect::new(async { Ok(vec![1, 2, 3]) }));
    let result = deferred.force().run().await;
    assert_eq!(result, Ok(vec![1, 2, 3]));
  }

  // Memoized tests
  #[tokio::test]
  async fn test_memoized_basic() {
    let counter = AtomicUsize::new(0);
    let effect = Effect::new(async move {
      counter.fetch_add(1, Ordering::SeqCst);
      Ok(42)
    });
    let memoized = Memoized::new(effect);

    assert_eq!(memoized.get().run().await, Ok(42));
    assert_eq!(memoized.get().run().await, Ok(42));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn test_memoized_error() {
    let counter = AtomicUsize::new(0);
    let effect = Effect::new(async move {
      counter.fetch_add(1, Ordering::SeqCst);
      Err(IoError::new(ErrorKind::Other, "test error"))
    });
    let memoized = Memoized::new(effect);

    let result = memoized.get().run().await;
    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    let result = memoized.get().run().await;
    assert!(result.is_err());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn test_memoized_concurrent() {
    let counter = AtomicUsize::new(0);
    let effect = Effect::new(async move {
      counter.fetch_add(1, Ordering::SeqCst);
      Ok(42)
    });
    let memoized = Memoized::new(effect);

    let mut handles = vec![];
    for _ in 0..10 {
      let memoized = memoized.clone();
      handles.push(tokio::spawn(async move { memoized.get().run().await }));
    }

    for handle in handles {
      assert_eq!(handle.await.unwrap(), Ok(42));
    }
    assert_eq!(counter.load(Ordering::SeqCst), 1);
  }

  #[tokio::test]
  async fn test_memoized_different_types() {
    let effect = Effect::new(async { Ok("hello".to_string()) });
    let memoized = Memoized::new(effect);
    assert_eq!(memoized.get().run().await, Ok("hello".to_string()));

    let effect = Effect::new(async { Ok(vec![1, 2, 3]) });
    let memoized = Memoized::new(effect);
    assert_eq!(memoized.get().run().await, Ok(vec![1, 2, 3]));
  }

  // Type alias tests
  #[tokio::test]
  async fn test_infallible() {
    let effect: Infallible<i32> = Effect::new(async { Ok(42) });
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_any_error() {
    let effect: AnyError<i32> = Effect::new(async { Ok(42) });
    assert_eq!(effect.run().await, Ok(42));

    let effect: AnyError<i32> =
      Effect::new(async { Err(Box::new(IoError::new(ErrorKind::Other, "test error"))) });
    let result = effect.run().await;
    assert!(result.is_err());
  }

  // Edge case tests
  #[tokio::test]
  async fn test_zero_sized_types() {
    let deferred = Deferred::new(|| Effect::new(async { Ok(()) }));
    assert_eq!(deferred.force().run().await, Ok(()));

    let effect = Effect::new(async { Ok(()) });
    let memoized = Memoized::new(effect);
    assert_eq!(memoized.get().run().await, Ok(()));
  }

  #[tokio::test]
  async fn test_large_types() {
    let data = vec![0u8; 1024 * 1024]; // 1MB of data
    let deferred = Deferred::new(|| Effect::new(async { Ok(data.clone()) }));
    let result = deferred.force().run().await;
    assert_eq!(result, Ok(data));

    let effect = Effect::new(async { Ok(data.clone()) });
    let memoized = Memoized::new(effect);
    assert_eq!(memoized.get().run().await, Ok(data));
  }

  // BoxFuture tests
  #[tokio::test]
  async fn test_box_future() {
    let future: BoxFuture<i32> = Box::pin(async { 42 });
    assert_eq!(future.await, 42);
  }
}
