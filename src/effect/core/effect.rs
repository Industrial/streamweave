//! Core Effect type implementation.
//!
//! This module provides the `Effect` type, which represents a computation that
//! may produce a value of type `T` or fail with an error of type `E`. The
//! computation is performed asynchronously, and the result is wrapped in a
//! `Result` type.

use std::{
  cmp::PartialEq,
  error::Error as StdError,
  fmt::Debug,
  future::Future,
  pin::Pin,
  sync::Arc,
  task::{Context, Poll},
};

use crate::effect::utils::shared::Shared;
use futures::FutureExt;
use tokio::sync::Mutex;

use super::future::EffectFuture;
use super::monad::Monad;

/// The Effect type represents an asynchronous computation that can fail
#[derive(Clone)]
pub struct Effect<T, E> {
  inner: Shared<Pin<Box<dyn Future<Output = Result<T, E>> + Send + Sync>>>,
}

impl<T: PartialEq, E: PartialEq> PartialEq for Effect<T, E> {
  fn eq(&self, other: &Self) -> bool {
    // Effects are considered equal if they are the same instance
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

impl<T: Eq, E: Eq> Eq for Effect<T, E> {}

impl<T, E> Effect<T, E>
where
  T: Debug + PartialEq + Send + Sync + 'static,
  E: Debug + PartialEq + Send + Sync + 'static,
{
  /// Creates a new Effect from a future
  pub fn new<F>(future: F) -> Self
  where
    F: Future<Output = Result<T, E>> + Send + Sync + 'static,
  {
    Effect {
      inner: Arc::new(Mutex::new(Box::pin(future))),
    }
  }

  /// Runs the effect and returns its result
  pub async fn run(&self) -> Result<T, E> {
    let mut future = self.inner.lock().await;
    future.as_mut().await
  }

  /// Converts the effect into a future.
  pub fn into_future(self) -> EffectFuture<T, E> {
    EffectFuture::new(self)
  }
}

impl<T, E> Future for Effect<T, E>
where
  T: Debug + PartialEq + Send + Sync + 'static,
  E: Debug + PartialEq + Send + Sync + 'static,
{
  type Output = Result<T, E>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut future = self.inner.try_lock().expect("Mutex poisoned");
    future.as_mut().poll(cx)
  }
}

impl<T, E> Monad for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: StdError + Send + Sync + 'static,
{
  type Inner = T;

  fn pure<A>(value: A) -> Self
  where
    A: Send + Sync + 'static,
  {
    Effect::new(async move { Ok(value) })
  }

  fn map<F, U>(self, f: F) -> Self
  where
    F: FnOnce(Self::Inner) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    Effect::new(async move {
      let value = self.run().await?;
      Ok(f(value))
    })
  }

  fn flat_map<F, U>(self, f: F) -> Self
  where
    F: FnOnce(Self::Inner) -> Self + Send + Sync + 'static,
  {
    Effect::new(async move {
      let value = self.run().await?;
      f(value).run().await
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::io::{self, Error as IoError, ErrorKind};
  use std::time::Duration;
  use tokio::time::sleep;

  // Basic functionality tests
  #[tokio::test]
  async fn test_effect_pure() {
    let effect: Effect<i32, IoError> = Effect::pure(42);
    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
  }

  #[tokio::test]
  async fn test_effect_map() {
    let effect: Effect<i32, IoError> = Effect::pure(42).map(|x| x * 2);
    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 84);
  }

  #[tokio::test]
  async fn test_effect_flat_map() {
    let effect: Effect<i32, IoError> = Effect::pure(42).flat_map(|x| Effect::pure(x * 2));
    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 84);
  }

  // Error handling tests
  #[tokio::test]
  async fn test_effect_error() {
    let effect: Effect<i32, IoError> =
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) });
    let result = effect.run().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Other);
    assert_eq!(err.to_string(), "test error");
  }

  #[tokio::test]
  async fn test_effect_error_propagation() {
    let effect: Effect<i32, IoError> =
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) }).map(|x| x * 2);
    let result = effect.run().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Other);
    assert_eq!(err.to_string(), "test error");
  }

  #[tokio::test]
  async fn test_effect_error_flat_map() {
    let effect: Effect<i32, IoError> =
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) })
        .flat_map(|x| Effect::pure::<i32>(x * 2));
    let result = effect.run().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Other);
    assert_eq!(err.to_string(), "test error");
  }

  // Async behavior tests
  #[tokio::test]
  async fn test_effect_async_delay() {
    let start = std::time::Instant::now();
    let effect: Effect<i32, IoError> = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(42)
    });
    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert!(start.elapsed() >= Duration::from_millis(100));
  }

  #[tokio::test]
  async fn test_effect_concurrent() {
    let effect1: Effect<i32, IoError> = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(1)
    });
    let effect2: Effect<i32, IoError> = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(2)
    });

    let start = std::time::Instant::now();
    let combined = effect1.flat_map(|x| effect2.map(move |y| x + y));
    let result = combined.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 3);
    assert!(start.elapsed() >= Duration::from_millis(200));
  }

  // Edge cases tests
  #[tokio::test]
  async fn test_effect_unit_type() {
    let effect: Effect<(), IoError> = Effect::pure(());
    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), ());
  }

  #[tokio::test]
  async fn test_effect_complex_type() {
    let effect: Effect<i32, IoError> =
      Effect::pure(vec![1, 2, 3]).map(|v| v.into_iter().sum::<i32>());
    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 6);
  }

  // Resource management tests
  #[tokio::test]
  async fn test_effect_resource_cleanup() {
    let mut cleanup_called = false;
    let effect: Effect<i32, IoError> = Effect::new(async move {
      let _guard = scopeguard::guard((), |_| cleanup_called = true);
      Ok(42)
    });

    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert!(cleanup_called);
  }

  // Combinator tests
  #[tokio::test]
  async fn test_effect_sequence() {
    let effects: Vec<Effect<i32, IoError>> =
      vec![Effect::pure(1), Effect::pure(2), Effect::pure(3)];

    let result = effects.into_iter().fold(Effect::pure(0), |acc, e| {
      acc.flat_map(move |x| e.map(move |y| x + y))
    });

    let final_result = result.run().await;
    assert!(final_result.is_ok());
    assert_eq!(final_result.unwrap(), 6);
  }

  // Type safety tests
  #[tokio::test]
  async fn test_effect_send_sync() {
    fn assert_send_sync<T: Send + Sync>(_: T) {}

    let effect: Effect<i32, IoError> = Effect::pure(42);
    assert_send_sync(effect);
  }

  #[tokio::test]
  async fn test_effect_static() {
    let effect: Effect<i32, IoError> = Effect::pure(42);
    let _: Effect<i32, IoError> = Effect::new(async { Ok(42) });
  }

  // Complex composition tests
  #[tokio::test]
  async fn test_effect_complex_composition() {
    let effect: Effect<String, IoError> = Effect::pure(1)
      .flat_map(|x| Effect::pure::<i32>(x + 1))
      .map(|x| x * 2)
      .flat_map(|x| Effect::pure::<i32>(x + 1))
      .map(|x| x.to_string());

    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "5");
  }

  // Error recovery tests
  #[tokio::test]
  async fn test_effect_error_recovery() {
    let effect: Effect<i32, IoError> =
      Effect::new(async { Err(IoError::new(ErrorKind::Other, "test error")) })
        .flat_map(|_| Effect::pure::<i32>(42))
        .map(|x| x * 2);

    let result = effect.run().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Other);
    assert_eq!(err.to_string(), "test error");
  }

  // Additional test cases for comprehensive coverage
  #[tokio::test]
  async fn test_effect_nested_flat_map() {
    let effect: Effect<i32, IoError> = Effect::pure(1)
      .flat_map(|x| Effect::pure::<i32>(x).map(|y| y + 1))
      .flat_map(|x| Effect::pure::<i32>(x).flat_map(|y| Effect::pure::<i32>(y * 2)));

    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 4);
  }

  #[tokio::test]
  async fn test_effect_error_chain() {
    let effect: Effect<i32, IoError> = Effect::pure(1)
      .flat_map(|_| Effect::new(async { Err(IoError::new(ErrorKind::Other, "error 1")) }))
      .map(|x| x + 1)
      .flat_map(|_| Effect::new(async { Err(IoError::new(ErrorKind::Other, "error 2")) }));

    let result = effect.run().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Other);
    assert_eq!(err.to_string(), "error 1");
  }

  #[tokio::test]
  async fn test_effect_different_types() {
    let effect: Effect<String, IoError> = Effect::pure(42)
      .map(|x| x.to_string())
      .flat_map(|s| Effect::pure::<String>(s + "!"))
      .map(|s| s.len())
      .map(|n| format!("Length: {}", n));

    let result = effect.run().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Length: 3");
  }

  #[tokio::test]
  async fn test_effect_timing_guarantees() {
    let effect1: Effect<(), IoError> = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(())
    });

    let effect2: Effect<(), IoError> = Effect::new(async {
      sleep(Duration::from_millis(50)).await;
      Ok(())
    });

    let start = std::time::Instant::now();
    let result = effect1.flat_map(|_| effect2).run().await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed >= Duration::from_millis(150));
  }

  #[tokio::test]
  async fn test_effect_into_future() {
    let effect: Effect<i32, IoError> = Effect::pure(42);
    let future = effect.into_future();
    let result = future.await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
  }
}
