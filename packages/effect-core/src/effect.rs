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

impl<T, E> Functor<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  type HigherSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn map<B, F>(mut self, mut f: F) -> Self::HigherSelf<B>
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
    Effect::new(async move { Ok(a) })
  }

  fn ap<B, F>(mut self, mut f: Self::HigherSelf<F>) -> Self::HigherSelf<B>
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
    Effect::new(async move { Ok(a) })
  }

  fn bind<B, F>(mut self, mut f: F) -> Self::HigherSelf<B>
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Duration;
  use tokio::time::sleep;

  #[tokio::test]
  async fn test_effect_creation() {
    let effect = Effect::new(async { Ok(42) });
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_pure() {
    let effect = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_error() {
    let effect = Effect::new(async { Err("error") });
    assert_eq!(effect.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_functor_map() {
    let effect = Effect::pure(42);
    let mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_functor_map_error() {
    let effect = Effect::new(async { Err("error") });
    let mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_functor_map_async() {
    let effect = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_applicative_pure() {
    let effect = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_applicative_ap() {
    let effect = Effect::pure(42);
    let function = Effect::pure(|x| x * 2);
    let applied = effect.ap(function);
    assert_eq!(applied.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_applicative_ap_error() {
    let effect = Effect::pure(42);
    let function = Effect::new(async { Err("error") });
    let applied = effect.ap(function);
    assert_eq!(applied.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_applicative_ap_async() {
    let effect = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let function = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(|x| x * 2)
    });
    let applied = effect.ap(function);
    assert_eq!(applied.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_monad_pure() {
    let effect = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_monad_bind() {
    let effect = Effect::pure(42);
    let bound = effect.bind(|x| Effect::pure(x * 2));
    assert_eq!(bound.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_monad_bind_error() {
    let effect = Effect::pure(42);
    let bound = effect.bind(|_| Effect::new(async { Err("error") }));
    assert_eq!(bound.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_monad_bind_async() {
    let effect = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let bound = effect.bind(|x| {
      Effect::new(async {
        sleep(Duration::from_millis(10)).await;
        Ok(x * 2)
      })
    });
    assert_eq!(bound.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_monad_laws() {
    // Left identity: pure a >>= f ≡ f a
    let a = 42;
    let f = |x| Effect::pure(x * 2);
    let left = Effect::pure(a).bind(f);
    let right = f(a);
    assert_eq!(left.run().await, right.run().await);

    // Right identity: m >>= pure ≡ m
    let m = Effect::pure(42);
    let left = m.bind(Effect::pure);
    let right = m;
    assert_eq!(left.run().await, right.run().await);

    // Associativity: (m >>= f) >>= g ≡ m >>= (\x -> f x >>= g)
    let m = Effect::pure(42);
    let f = |x| Effect::pure(x * 2);
    let g = |x| Effect::pure(x + 1);
    let left = m.bind(f).bind(g);
    let right = m.bind(|x| f(x).bind(g));
    assert_eq!(left.run().await, right.run().await);
  }

  #[tokio::test]
  async fn test_effect_functor_laws() {
    // Identity: map id ≡ id
    let effect = Effect::pure(42);
    let mapped = effect.map(|x| x);
    assert_eq!(mapped.run().await, effect.run().await);

    // Composition: map (f . g) ≡ map f . map g
    let effect = Effect::pure(42);
    let f = |x| x * 2;
    let g = |x| x + 1;
    let left = effect.map(|x| f(g(x)));
    let right = effect.map(g).map(f);
    assert_eq!(left.run().await, right.run().await);
  }

  #[tokio::test]
  async fn test_effect_applicative_laws() {
    // Identity: pure id <*> v ≡ v
    let v = Effect::pure(42);
    let id = Effect::pure(|x| x);
    let left = v.ap(id);
    assert_eq!(left.run().await, v.run().await);

    // Homomorphism: pure f <*> pure x ≡ pure (f x)
    let f = |x| x * 2;
    let x = 42;
    let left = Effect::pure(x).ap(Effect::pure(f));
    let right = Effect::pure(f(x));
    assert_eq!(left.run().await, right.run().await);

    // Interchange: u <*> pure y ≡ pure ($ y) <*> u
    let u = Effect::pure(|x| x * 2);
    let y = 42;
    let left = Effect::pure(y).ap(u);
    let right = u.ap(Effect::pure(|f| f(y)));
    assert_eq!(left.run().await, right.run().await);
  }

  #[tokio::test]
  async fn test_effect_complex_composition() {
    let effect = Effect::pure(42);
    let result = effect
      .map(|x| x * 2)
      .bind(|x| Effect::pure(x + 1))
      .ap(Effect::pure(|x| x * 3));
    assert_eq!(result.run().await, Ok(255));
  }

  #[tokio::test]
  async fn test_effect_error_propagation() {
    let effect = Effect::new(async { Err("error") });
    let result = effect
      .map(|x| x * 2)
      .bind(|x| Effect::pure(x + 1))
      .ap(Effect::pure(|x| x * 3));
    assert_eq!(result.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_async_composition() {
    let effect = Effect::new(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let result = effect
      .map(|x| x * 2)
      .bind(|x| {
        Effect::new(async {
          sleep(Duration::from_millis(10)).await;
          Ok(x + 1)
        })
      })
      .ap(Effect::new(async {
        sleep(Duration::from_millis(10)).await;
        Ok(|x| x * 3)
      }));
    assert_eq!(result.run().await, Ok(255));
  }
}
