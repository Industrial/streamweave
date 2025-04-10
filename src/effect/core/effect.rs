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

use super::monad::Monad;

/// The Effect type represents an asynchronous computation that can yield a value of type T
/// or fail with an error of type E.
///
/// The type parameters T and E must satisfy the following bounds:
/// - T: PartialEq + Send + Sync + 'static
/// - E: Debug + Send + Sync + 'static
pub struct Effect<T, E> {
  inner: Pin<Box<dyn Future<Output = Result<T, E>> + Send + Sync>>,
}

impl<T, E> Effect<T, E> {
  /// Creates a new Effect from a future
  pub fn new<F>(future: F) -> Self
  where
    F: Future<Output = Result<T, E>> + Send + Sync + 'static,
  {
    Effect {
      inner: Box::pin(future),
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

  /// Runs the effect and returns its result
  pub async fn run(&mut self) -> Result<T, E> {
    self.inner.as_mut().await
  }
}

impl<T, E> Future for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type Output = Result<T, E>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    self.inner.as_mut().poll(cx)
  }
}

impl<T, E> Monad for Effect<T, E>
where
  T: Debug + PartialEq + Send + Sync + 'static,
  E: Debug + PartialEq + Send + Sync + 'static,
{
  type Inner = T;

  fn pure<A>(value: A) -> Self
  where
    A: Debug + PartialEq + Send + Sync + 'static,
    T: From<A>,
  {
    Effect::new(async move { Ok(T::from(value)) })
  }

  fn map<F, U>(self, f: F) -> Self
  where
    F: FnOnce(Self::Inner) -> U + Send + Sync + 'static,
    U: Debug + PartialEq + Send + Sync + 'static,
    T: From<U>,
  {
    Effect::new(async move {
      let value = self.run().await?;
      Ok(T::from(f(value)))
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
