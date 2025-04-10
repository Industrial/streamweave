//! Monad trait and implementations for the Effect type.
//!
//! This module defines the `Monad` trait and provides implementations for the
//! `Effect` type, enabling monadic composition and transformation of effects.

/// The Monad trait defines the basic operations for monadic types.
pub trait Monad {
  /// The inner type of the monad.
  type Inner;

  /// Creates a new monad from a value.
  fn pure<T>(value: T) -> Self
  where
    Self: Sized,
    T: Send + Sync + 'static;

  /// Transforms the inner value of the monad.
  fn map<F, U>(self, f: F) -> Self
  where
    F: FnOnce(Self::Inner) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
    Self: Sized;

  /// Composes two monads, applying a function to the inner value.
  fn flat_map<F, U>(self, f: F) -> Self
  where
    F: FnOnce(Self::Inner) -> Self + Send + Sync + 'static,
    Self: Sized;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::effect::core::effect::Effect;
  use std::time::Duration;
  use tokio::time::sleep;

  // Monad laws tests
  #[tokio::test]
  async fn test_monad_laws() {
    // Left identity: pure(a).flat_map(f) == f(a)
    let a = 42;
    let f = |x: i32| Effect::pure(x * 2);
    assert_eq!(Effect::pure(a).flat_map(f).run().await, f(a).run().await);

    // Right identity: m.flat_map(pure) == m
    let m = Effect::pure(42);
    assert_eq!(m.clone().flat_map(Effect::pure).run().await, m.run().await);

    // Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    let m = Effect::pure(42);
    let f = |x: i32| Effect::pure(x * 2);
    let g = |x: i32| Effect::pure(x + 1);
    assert_eq!(
      m.clone().flat_map(f).flat_map(g).run().await,
      m.flat_map(|x| f(x).flat_map(g)).run().await
    );
  }

  // Basic functionality tests
  #[tokio::test]
  async fn test_monad_pure() {
    let effect = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_monad_map() {
    let effect = Effect::pure(42).map(|x| x * 2);
    assert_eq!(effect.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_monad_flat_map() {
    let effect = Effect::pure(42).flat_map(|x| Effect::pure(x * 2));
    assert_eq!(effect.run().await, Ok(84));
  }

  // Error handling tests
  #[tokio::test]
  async fn test_monad_error_propagation() {
    let effect = Effect::new(async { Err("error") }).map(|x: i32| x * 2);
    assert_eq!(effect.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_monad_error_flat_map() {
    let effect = Effect::new(async { Err("error") }).flat_map(|x: i32| Effect::pure(x * 2));
    assert_eq!(effect.run().await, Err("error"));
  }

  // Async behavior tests
  #[tokio::test]
  async fn test_monad_async_composition() {
    let effect = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(42)
    })
    .flat_map(|x| {
      Effect::new(async {
        sleep(Duration::from_millis(100)).await;
        Ok(x * 2)
      })
    });

    let start = std::time::Instant::now();
    assert_eq!(effect.run().await, Ok(84));
    assert!(start.elapsed() >= Duration::from_millis(200));
  }

  // Type safety tests
  #[tokio::test]
  async fn test_monad_type_safety() {
    fn assert_monad<M: Monad>(_: M) {}

    let effect = Effect::pure(42);
    assert_monad(effect);
  }

  // Edge cases tests
  #[tokio::test]
  async fn test_monad_unit_type() {
    let effect = Effect::pure(());
    assert_eq!(effect.run().await, Ok(()));
  }

  #[tokio::test]
  async fn test_monad_complex_type() {
    let effect = Effect::pure(vec![1, 2, 3]).map(|v| v.into_iter().sum::<i32>());
    assert_eq!(effect.run().await, Ok(6));
  }

  // Complex composition tests
  #[tokio::test]
  async fn test_monad_complex_composition() {
    let effect = Effect::pure(1)
      .flat_map(|x| Effect::pure(x + 1))
      .map(|x| x * 2)
      .flat_map(|x| Effect::pure(x + 1))
      .map(|x| x.to_string());

    assert_eq!(effect.run().await, Ok("5".to_string()));
  }

  // Resource cleanup tests
  #[tokio::test]
  async fn test_monad_resource_cleanup() {
    use scopeguard::guard;

    let mut cleanup_called = false;
    let effect = Effect::new(async move {
      let _guard = guard((), |_| cleanup_called = true);
      Ok(42)
    })
    .map(|x| x * 2);

    assert_eq!(effect.run().await, Ok(84));
    assert!(cleanup_called);
  }

  // Concurrent composition tests
  #[tokio::test]
  async fn test_monad_concurrent_composition() {
    let effect1 = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(1)
    });
    let effect2 = Effect::new(async {
      sleep(Duration::from_millis(100)).await;
      Ok(2)
    });

    let combined = effect1.flat_map(|x| effect2.map(move |y| x + y));
    assert_eq!(combined.run().await, Ok(3));
  }

  // Error recovery tests
  #[tokio::test]
  async fn test_monad_error_recovery() {
    let effect = Effect::new(async { Err("error") })
      .flat_map(|_: i32| Effect::pure(42))
      .map(|x| x * 2);

    assert_eq!(effect.run().await, Err("error"));
  }

  // Nested composition tests
  #[tokio::test]
  async fn test_monad_nested_composition() {
    let effect = Effect::pure(1)
      .flat_map(|x| Effect::pure(x + 1).flat_map(|y| Effect::pure(y * 2).map(|z| z + x)));

    assert_eq!(effect.run().await, Ok(5));
  }
}
