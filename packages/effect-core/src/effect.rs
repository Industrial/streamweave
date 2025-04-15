//! Core Effect type implementation.
//!
//! This module provides the `Effect` type, which represents a computation that
//! may produce a value of type `T` or fail with an error of type `E`. The
//! computation is performed asynchronously, and the result is wrapped in a
//! `Result` type.

use std::{
  cmp::PartialEq,
  fmt::Debug,
  future::Future,
  pin::Pin,
  sync::Arc,
  task::{Context, Poll},
};
use tokio::sync::Mutex;

use super::applicative::Applicative;
use super::functor::Functor;
use super::monad::Monad;
use crate::monoid::Monoid;
use crate::semigroup::Semigroup;
use std::error::Error;

/// Type alias for an Effect that never fails
pub type Infallible<T> = Effect<T, std::convert::Infallible>;

/// Type alias for an Effect that can fail with any error type
pub type AnyError<T> = Effect<T, Box<dyn std::error::Error + Send + Sync>>;

/// The Effect type represents an asynchronous computation that can yield a value of type T
/// or fail with an error of type E.
///
/// The type parameters T and E must satisfy the following bounds:
/// - T: Send + Sync + 'static
/// - E: Send + Sync + 'static
pub struct Effect<T, E> {
  inner: Arc<Mutex<Pin<Box<dyn Future<Output = Result<T, E>> + Send + Sync + 'static>>>>,
}

impl<T, E> Effect<T, E> {
  /// Creates a new Effect from a future
  pub fn new<F>(future: F) -> Self
  where
    F: Future<Output = Result<T, E>> + Send + Sync + 'static,
  {
    Effect {
      inner: Arc::new(Mutex::new(Box::pin(future))),
    }
  }

  /// Creates a new Effect that immediately yields a value
  pub fn pure(value: T) -> Self
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move { Ok(value) })
  }

  /// Creates a new Effect that immediately yields an error
  pub fn error(error: E) -> Self
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move { Err(error) })
  }

  /// Runs the effect and returns its result
  pub async fn run(self) -> Result<T, E>
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    let mut guard = self.inner.lock().await;
    (&mut *guard).as_mut().await
  }

  /// Maps an error to a different error type
  pub fn map_error<F, E2>(self, f: F) -> Effect<T, E2>
  where
    F: FnOnce(E) -> E2 + Send + Sync + 'static,
    E2: Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move {
      match self.run().await {
        Ok(value) => Ok(value),
        Err(error) => Err(f(error)),
      }
    })
  }

  /// Handles an error by converting it into a new Effect
  pub fn handle_error<F, G>(self, f: F) -> Effect<T, G>
  where
    F: FnOnce(E) -> Effect<T, G> + Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
    G: Send + Sync + 'static,
  {
    Effect::new(async move {
      match self.run().await {
        Ok(value) => Ok(value),
        Err(e) => f(e).run().await,
      }
    })
  }

  /// Chains a function that returns an Effect after this one
  pub fn and_then<F, B>(self, f: F) -> Effect<B, E>
  where
    F: FnOnce(T) -> Effect<B, E> + Send + Sync + 'static,
    B: Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move {
      let value = self.run().await?;
      f(value).run().await
    })
  }

  /// Handles an error by converting it into a new Effect
  pub fn or_else<F>(self, f: F) -> Effect<T, E>
  where
    F: FnOnce(E) -> Effect<T, E> + Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    self.handle_error(f)
  }

  pub fn ok(value: T) -> Self
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::pure(value)
  }

  pub fn err(error: E) -> Self
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::error(error)
  }

  /// Returns the first successful effect, or the second if the first fails
  pub fn or(self, other: Self) -> Self
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move {
      match self.run().await {
        Ok(value) => Ok(value),
        Err(_) => other.run().await,
      }
    })
  }
}

impl<T> Effect<T, std::convert::Infallible> {
  /// Creates a new infallible effect
  pub fn infallible(value: T) -> Self
  where
    T: Send + Sync + 'static,
  {
    Effect::new(async move { Ok(value) })
  }
}

impl<T> Effect<T, Box<dyn std::error::Error + Send + Sync>> {
  /// Creates a new effect that can fail with any error
  pub fn any_error(value: T) -> Self
  where
    T: Send + Sync + 'static,
  {
    Effect::new(async move { Ok(value) })
  }
}

impl<T, E> Future for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type Output = Result<T, E>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut future = match self.inner.try_lock() {
      Ok(guard) => guard,
      Err(_) => return Poll::Pending,
    };
    future.as_mut().poll(cx)
  }
}

impl<T, E> Functor<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    Effect::new(async move {
      let result = self.run().await?;
      Ok(f(result))
    })
  }
}

impl<T, E> Applicative<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn pure(a: T) -> Self {
    Effect::pure(a)
  }

  fn ap<B, F>(self, mut f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut f = f.run().await?;
      let a = self.run().await?;
      Ok(f(a))
    })
  }
}

impl<T, E> Monad<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn pure(a: T) -> Self::HigherSelf<T> {
    Effect::pure(a)
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    Effect::new(async move {
      let a = self.run().await?;
      f(a).run().await
    })
  }
}

impl<T, E> Effect<T, E>
where
  T: Debug + PartialEq + Send + Sync + 'static,
  E: Debug + PartialEq + Send + Sync + 'static,
{
  /// Creates a new effect with the default error type
  pub fn result(value: T) -> Self {
    Self::new(async move { Ok(value) })
  }

  /// Creates a new effect from a future with the default error type
  pub fn from_future<F>(future: F) -> Self
  where
    F: Future<Output = Result<T, E>> + Send + Sync + 'static,
  {
    Self::new(future)
  }
}

impl<T, E> Clone for Effect<T, E> {
  fn clone(&self) -> Self {
    Effect {
      inner: Arc::clone(&self.inner),
    }
  }
}

impl<T, E> Semigroup for Effect<T, E>
where
  T: Send + Sync + Clone + Default + 'static,
  E: Error + Send + Sync + 'static,
{
  fn combine(self, other: Self) -> Self {
    Effect::new(async move {
      match self.run().await {
        Ok(val) => Ok(val),
        Err(_) => other.run().await,
      }
    })
  }
}

impl<T, E> Monoid for Effect<T, E>
where
  T: Send + Sync + Clone + Default + 'static,
  E: Error + Send + Sync + 'static,
{
  fn empty() -> Self {
    Effect::pure(T::default())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::time::Duration;
  use tokio::time::sleep;

  fn double(x: i32) -> i32 {
    x * 2
  }

  fn triple(x: i32) -> i32 {
    x * 3
  }

  fn identity(x: i32) -> i32 {
    x
  }

  fn apply_to(y: i32) -> impl Fn(fn(i32) -> i32) -> i32 {
    move |f| f(y)
  }

  proptest! {
    #[test]
    fn test_effect_creation(x: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(effect.run());
      prop_assert!(matches!(result, Ok(val) if val == x));
    }

    #[test]
    fn test_effect_error(e in "\\PC*") {
      let error = e.clone();
      let effect = Effect::<i32, String>::error(e);
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(effect.run());
      prop_assert!(matches!(result, Err(err) if err == error));
    }

    #[test]
    fn test_effect_map(x: i32, y: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let mapped = effect.map(move |a| a + y);
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(mapped.run());
      prop_assert!(matches!(result, Ok(val) if val == x + y));
    }

    #[test]
    fn test_effect_ap(x: i32, y: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let function: Effect<Box<dyn FnMut(i32) -> i32 + Send + Sync>, std::io::Error> =
        Effect::pure(Box::new(move |a: i32| a + y) as Box<dyn FnMut(i32) -> i32 + Send + Sync>);
      let applied = effect.ap(function);
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(applied.run());
      prop_assert!(matches!(result, Ok(val) if val == x + y));
    }

    #[test]
    fn test_effect_bind(x: i32, y: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let bound = effect.bind(move |a| Effect::pure(a + y));
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(bound.run());
      prop_assert!(matches!(result, Ok(val) if val == x + y));
    }

    #[test]
    fn test_effect_async(x: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::new(async move {
        sleep(Duration::from_millis(10)).await;
        Ok(x)
      });
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(effect.run());
      prop_assert!(matches!(result, Ok(val) if val == x));
    }

    #[test]
    fn test_effect_error_handling(e in "\\PC*") {
      let effect = Effect::<i32, String>::error(e);
      let handled: Effect<i32, String> = effect.handle_error(|_| Effect::pure(42));
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(handled.run());
      prop_assert!(matches!(result, Ok(42)));
    }

    #[test]
    fn test_effect_map_error(e in "\\PC*") {
      let effect = Effect::<i32, String>::error(e);
      let mapped = effect.map_error(|_| "new error".to_string());
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(mapped.run());
      prop_assert!(matches!(result, Err(err) if err == "new error"));
    }

    #[test]
    fn test_effect_and_then(x: i32, y: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let chained = effect.and_then(move |a| Effect::pure(a + y));
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(chained.run());
      prop_assert!(matches!(result, Ok(val) if val == x + y));
    }

    #[test]
    fn test_effect_or_else(x: i32) {
      let effect = Effect::<i32, String>::error("error".to_string());
      let recovered = effect.or_else(move |_| Effect::pure(x));
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(recovered.run());
      prop_assert!(matches!(result, Ok(val) if val == x));
    }

    #[test]
    fn test_effect_semigroup(x: i32, y: i32) {
      let effect1: Effect<i32, std::io::Error> = Effect::pure(x);
      let effect2: Effect<i32, std::io::Error> = Effect::pure(y);
      let combined = effect1.combine(effect2);
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(combined.run());
      prop_assert!(matches!(result, Ok(val) if val == x));
    }

    #[test]
    fn test_effect_semigroup_error(x: i32) {
      let effect1 = Effect::<i32, std::io::Error>::error(std::io::Error::new(std::io::ErrorKind::Other, "error1"));
      let effect2: Effect<i32, std::io::Error> = Effect::pure(x);
      let combined = effect1.combine(effect2);
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(combined.run());
      prop_assert!(matches!(result, Ok(val) if val == x));
    }

    #[test]
    fn test_effect_monoid(x: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let empty = Effect::<i32, std::io::Error>::empty();
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result = runtime.block_on(empty.combine(effect).run());
      prop_assert!(matches!(result, Ok(val) if val == x));
    }

    #[test]
    fn test_effect_clone(x: i32) {
      let effect: Effect<i32, std::io::Error> = Effect::pure(x);
      let cloned = effect.clone();
      let runtime = tokio::runtime::Runtime::new().unwrap();
      let result1 = runtime.block_on(effect.run());
      let result2 = runtime.block_on(cloned.run());
      prop_assert!(matches!(result1, Ok(val1) if matches!(result2, Ok(val2) if val1 == val2)));
    }
  }
}
