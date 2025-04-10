//! Future implementation for the Effect type.
//!
//! This module provides the `EffectFuture` type and its implementation of the
//! `Future` trait, enabling async execution of effects.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::effect::Effect;

/// A future that represents an asynchronous effect computation.
pub struct EffectFuture<T, E> {
  inner: Pin<Box<dyn Future<Output = Result<T, E>> + Send + Sync>>,
}

impl<T, E> EffectFuture<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  /// Creates a new `EffectFuture` from an effect.
  pub fn new(effect: Effect<T, E>) -> Self {
    Self {
      inner: Box::pin(effect.run()),
    }
  }
}

impl<T, E> Future for EffectFuture<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type Output = Result<T, E>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.inner.as_mut().poll(cx)
  }
}

#[cfg(test)]
mod tests {
  use super::super::effect::Effect;
  use super::*;
  use std::time::Duration;
  use tokio::time::sleep;

  // Basic functionality tests
  #[tokio::test]
  async fn test_effect_future() {
    let effect = Effect::pure(42);
    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Ok(42));
  }

  // Error handling tests
  #[tokio::test]
  async fn test_effect_future_error() {
    let effect = Effect::new(async { Err("error") });
    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Err("error"));
  }

  // Async behavior tests
  #[tokio::test]
  async fn test_effect_future_async() {
    let start = std::time::Instant::now();
    let effect = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(42)
    });
    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Ok(42));
    assert!(start.elapsed() >= Duration::from_millis(100));
  }

  // Type safety tests
  #[tokio::test]
  async fn test_effect_future_send_sync() {
    fn assert_send_sync<T: Send + Sync>(_: T) {}

    let effect = Effect::pure(42);
    let future = EffectFuture::new(effect);
    assert_send_sync(future);
  }

  // Edge cases tests
  #[tokio::test]
  async fn test_effect_future_unit_type() {
    let effect = Effect::pure(());
    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Ok(()));
  }

  #[tokio::test]
  async fn test_effect_future_complex_type() {
    let effect = Effect::pure(vec![1, 2, 3]);
    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Ok(vec![1, 2, 3]));
  }

  // Cancellation tests
  #[tokio::test]
  async fn test_effect_future_cancellation() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let effect = Effect::new(async move {
      sleep(Duration::from_millis(100)).await;
      let _ = tx.send(());
      Ok(42)
    });

    let future = EffectFuture::new(effect);
    drop(future);

    // Wait for the effect to complete
    let _ = rx.await;
  }

  // Concurrent tests
  #[tokio::test]
  async fn test_effect_future_concurrent() {
    let effect1 = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(1)
    });
    let effect2 = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(2)
    });

    let future1 = EffectFuture::new(effect1);
    let future2 = EffectFuture::new(effect2);

    let (result1, result2) = tokio::join!(future1, future2);
    assert_eq!(result1, Ok(1));
    assert_eq!(result2, Ok(2));
  }

  // Polling tests
  #[tokio::test]
  async fn test_effect_future_polling() {
    use std::pin::Pin;
    use std::task::Poll;

    let effect = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(42)
    });

    let mut future = EffectFuture::new(effect);
    let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());

    // First poll should be pending
    assert!(matches!(Pin::new(&mut future).poll(&mut cx), Poll::Pending));

    // After waiting, should be ready
    assert_eq!(future.await, Ok(42));
  }

  // Resource cleanup tests
  #[tokio::test]
  async fn test_effect_future_resource_cleanup() {
    use scopeguard::guard;

    let mut cleanup_called = false;
    let effect = Effect::new(async move {
      let _guard = guard((), |_| cleanup_called = true);
      Ok(42)
    });

    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Ok(42));
    assert!(cleanup_called);
  }

  // Error propagation tests
  #[tokio::test]
  async fn test_effect_future_error_propagation() {
    let effect = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Err("error")
    });

    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Err("error"));
  }

  // Complex composition tests
  #[tokio::test]
  async fn test_effect_future_complex_composition() {
    let effect = Effect::pure(1)
      .flat_map(|x| Effect::pure(x + 1))
      .map(|x| x * 2);

    let future = EffectFuture::new(effect);
    assert_eq!(future.await, Ok(4));
  }
}
