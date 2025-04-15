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
  task::{Context, Poll},
};

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

/// A type that represents an asynchronous computation that can yield a value of type `T` or fail with an error of type `E`.
///
/// # Type Parameters
///
/// - T: The type of the successful result
/// - E: The type of the error
///
/// # Trait Bounds
///
/// - T: Send + Sync + 'static
/// - E: Send + Sync + 'static
pub struct Effect<T, E> {
  inner: Pin<Box<dyn Future<Output = Result<T, E>> + Send + Sync>>,
}

impl<T, E> Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  /// Creates a new Effect from a future
  pub fn new<F>(future: F) -> Self
  where
    F: Future<Output = Result<T, E>> + Send + Sync + 'static,
  {
    Self {
      inner: Box::pin(future),
    }
  }

  /// Creates a new Effect that always succeeds with the given value
  pub fn pure(value: T) -> Self {
    Self::new(std::future::ready(Ok(value)))
  }

  /// Creates a new Effect that always fails with the given error
  pub fn error(error: E) -> Self {
    Self::new(std::future::ready(Err(error)))
  }

  /// Runs the Effect and returns its result
  pub async fn run(self) -> Result<T, E> {
    self.inner.await
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
    T: Clone,
    E: Send + Sync + 'static,
  {
    Effect::pure(value)
  }

  pub fn err(error: E) -> Self
  where
    T: Send + Sync + 'static,
    E: Clone,
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

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.get_mut().inner.as_mut().poll(cx)
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

  fn ap<B, F>(self, f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
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
  E: Error + Send + Sync + Clone + Default + 'static,
{
  fn empty() -> Self {
    Effect::error(E::default())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[derive(Debug, PartialEq, Clone)]
  struct TestError(String);
  impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.0)
    }
  }
  impl std::error::Error for TestError {}
  impl Default for TestError {
    fn default() -> Self {
      TestError("default error".to_string())
    }
  }

  proptest! {
    #[test]
    fn test_effect_creation(x: i32) {
      let effect: Effect<i32, TestError> = Effect::pure(x);
      let result = futures::executor::block_on(effect.run());
      prop_assert_eq!(result, Ok(x));
    }

    #[test]
    fn test_effect_error(x: i32) {
      let effect: Effect<i32, TestError> = Effect::error(TestError(x.to_string()));
      let result = futures::executor::block_on(effect.run());
      prop_assert_eq!(result, Err(TestError(x.to_string())));
    }

    #[test]
    fn test_effect_map(x: i32) {
      let effect: Effect<i32, TestError> = Effect::pure(x);
      let mapped = effect.map(|x| x.wrapping_mul(2));
      let result = futures::executor::block_on(mapped.run());
      prop_assert_eq!(result, Ok(x.wrapping_mul(2)));
    }

    #[test]
    fn test_effect_ap(x: i32) {
      let effect: Effect<i32, TestError> = Effect::pure(x);
      let function: Effect<Box<dyn FnMut(i32) -> i32 + Send + Sync>, TestError> =
        Effect::pure(Box::new(|x| x.wrapping_mul(2)));
      let applied = effect.ap(function);
      let result = futures::executor::block_on(applied.run());
      prop_assert_eq!(result, Ok(x.wrapping_mul(2)));
    }

    #[test]
    fn test_effect_bind(x: i32) {
      let effect: Effect<i32, TestError> = Effect::pure(x);
      let bound = effect.bind(|x| Effect::pure(x.wrapping_mul(2)));
      let result = futures::executor::block_on(bound.run());
      prop_assert_eq!(result, Ok(x.wrapping_mul(2)));
    }

    #[test]
    fn test_effect_error_handling(_x: i32) {
      let effect: Effect<i32, TestError> = Effect::error(TestError("test".to_string()));
      let handled: Effect<i32, TestError> = effect.handle_error(|_| Effect::pure(42));
      let result = futures::executor::block_on(handled.run());
      prop_assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_effect_map_error(x: i32) {
      let x_owned = x;
      let effect: Effect<i32, TestError> = Effect::error(TestError("test".to_string()));
      let mapped = effect.map_error(move |_| TestError(x_owned.to_string()));
      let result = futures::executor::block_on(mapped.run());
      prop_assert_eq!(result, Err(TestError(x_owned.to_string())));
    }

    #[test]
    fn test_effect_and_then(x: i32) {
      let effect: Effect<i32, TestError> = Effect::pure(x);
      let and_then = effect.and_then(|x| Effect::pure(x.wrapping_mul(2)));
      let result = futures::executor::block_on(and_then.run());
      prop_assert_eq!(result, Ok(x.wrapping_mul(2)));
    }

    #[test]
    fn test_effect_or_else(x: i32) {
      let effect: Effect<i32, TestError> = Effect::error(TestError("test".to_string()));
      let or_else = effect.or_else(move |_| Effect::pure(x));
      let result = futures::executor::block_on(or_else.run());
      prop_assert_eq!(result, Ok(x));
    }

    #[test]
    fn test_effect_semigroup(x: i32, y: i32) {
      let effect1: Effect<i32, TestError> = Effect::pure(x);
      let effect2: Effect<i32, TestError> = Effect::pure(y);
      let combined = effect1.combine(effect2);
      let result = futures::executor::block_on(combined.run());
      prop_assert_eq!(result, Ok(x));
    }
  }
}
