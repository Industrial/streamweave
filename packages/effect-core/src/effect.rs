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
pub struct Effect<T, E = std::io::Error> {
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

  /// Creates a new Effect that immediately yields an error
  pub fn error(error: E) -> Self
  where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move { Err(error) })
  }

  /// Runs the effect and returns its result
  pub async fn run(&mut self) -> Result<T, E> {
    self.inner.as_mut().await
  }

  /// Maps an error to a different error type
  pub fn map_error<F, E2>(mut self, f: F) -> Effect<T, E2>
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
  pub fn handle_error<F>(mut self, f: F) -> Effect<T, E>
  where
    F: FnOnce(E) -> Effect<T, E> + Send + Sync + 'static,
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
  {
    Effect::new(async move {
      match self.run().await {
        Ok(value) => Ok(value),
        Err(error) => f(error).run().await,
      }
    })
  }

  /// Chains a function that returns an Effect after this one
  pub fn and_then<F, B>(mut self, f: F) -> Effect<B, E>
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

  fn double(x: i32) -> i32 {
    x * 2
  }

  fn triple(x: i32) -> i32 {
    x * 3
  }

  fn increment(x: i32) -> i32 {
    x + 1
  }

  fn identity(x: i32) -> i32 {
    x
  }

  fn apply_to(y: i32) -> impl Fn(fn(i32) -> i32) -> i32 {
    move |f| f(y)
  }

  #[tokio::test]
  async fn test_effect_creation() {
    let mut effect: Infallible<i32> = Effect::infallible(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_pure() {
    let mut effect: Infallible<i32> = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_error() {
    let mut effect: Effect<i32, &str> = Effect::error("error");
    assert_eq!(effect.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_functor_map() {
    let effect: Infallible<i32> = Effect::pure(42);
    let mut mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_functor_map_error() {
    let effect: Effect<i32, &str> = Effect::error("error");
    let mut mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Err("error"));
  }

  #[tokio::test]
  async fn test_effect_functor_map_async() {
    let effect: Infallible<i32> = Effect::from_future(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let mut mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_applicative_pure() {
    let mut effect: Infallible<i32> = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_applicative_ap() {
    let effect: Infallible<i32> = Effect::pure(42);
    let function: Infallible<fn(i32) -> i32> = Effect::pure(|x| x * 2);
    let mut applied = effect.ap(function);
    assert_eq!(applied.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_applicative_ap_error() {
    let effect: Effect<i32, std::io::Error> = Effect::pure(42);
    let function: Effect<fn(i32) -> i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "error"));
    let mut applied = effect.ap(function);
    assert!(applied.run().await.is_err());
  }

  #[tokio::test]
  async fn test_effect_applicative_ap_async() {
    let effect: Infallible<i32> = Effect::from_future(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let function: Infallible<fn(i32) -> i32> = Effect::pure(double);
    let mut applied = effect.ap(function);
    assert_eq!(applied.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_monad_pure() {
    let mut effect: Infallible<i32> = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));
  }

  #[tokio::test]
  async fn test_effect_monad_bind() {
    let effect: Infallible<i32> = Effect::pure(42);
    let mut bound = effect.bind(|x| Effect::pure(x * 2));
    assert_eq!(bound.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_monad_bind_error() {
    let effect: Effect<i32, std::io::Error> = Effect::pure(42);
    let mut bound = effect.bind(|_| {
      Effect::<i32, std::io::Error>::error(std::io::Error::new(std::io::ErrorKind::Other, "error"))
    });
    assert!(bound.run().await.is_err());
  }

  #[tokio::test]
  async fn test_effect_monad_bind_async() {
    let effect: Infallible<i32> = Effect::from_future(async {
      sleep(Duration::from_millis(10)).await;
      Ok(42)
    });
    let mut bound = effect.bind(|x| {
      let x = x.clone();
      Effect::from_future(async move {
        sleep(Duration::from_millis(10)).await;
        Ok(double(x))
      })
    });
    assert_eq!(bound.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_monad_laws() {
    // Left identity: pure(a).bind(f) == f(a)
    let a = 42;
    let f = |x: i32| Effect::<i32, std::convert::Infallible>::pure(x + 1);
    let mut left = Effect::<i32, std::convert::Infallible>::pure(a).bind(f);
    let mut right = f(a);
    assert_eq!(left.run().await, right.run().await);

    // Right identity: m.bind(pure) == m
    let m = Effect::<i32, std::convert::Infallible>::pure(42);
    let pure = |x: i32| Effect::<i32, std::convert::Infallible>::pure(x);
    let mut bound = m.bind(pure);
    let mut m_clone = Effect::<i32, std::convert::Infallible>::pure(42);
    assert_eq!(bound.run().await, m_clone.run().await);

    // Associativity: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
    let m = Effect::<i32, std::convert::Infallible>::pure(42);
    let f = |x: i32| Effect::<i32, std::convert::Infallible>::pure(x + 1);
    let g = |x: i32| Effect::<i32, std::convert::Infallible>::pure(x * 2);
    let mut left = m.bind(f).bind(g);
    let m2 = Effect::<i32, std::convert::Infallible>::pure(42);
    let mut right = m2.bind(move |x| f(x).bind(g));
    assert_eq!(left.run().await, right.run().await);
  }

  #[tokio::test]
  async fn test_effect_functor_laws() {
    // Identity: map id ≡ id
    let effect = Effect::<i32, std::convert::Infallible>::pure(42);
    let mut mapped = effect.map(|x| x);
    let mut effect_clone = Effect::<i32, std::convert::Infallible>::pure(42);
    assert_eq!(mapped.run().await, effect_clone.run().await);

    // Composition: map (f . g) ≡ map f . map g
    let effect = Effect::<i32, std::convert::Infallible>::pure(42);
    let f = |x| x * 2;
    let g = |x| x + 1;
    let mut left = effect.map(move |x| f(g(x)));
    let effect2 = Effect::<i32, std::convert::Infallible>::pure(42);
    let mut right = effect2.map(move |x| g(x)).map(move |x| f(x));
    assert_eq!(left.run().await, right.run().await);
  }

  #[tokio::test]
  async fn test_effect_applicative_laws() {
    // Identity: pure id <*> v ≡ v
    let mut v: Infallible<i32> = Effect::pure(42);
    let id: Infallible<fn(i32) -> i32> = Effect::pure(identity);
    let mut left = v.ap(id);
    let mut v_clone: Infallible<i32> = Effect::pure(42);
    assert_eq!(left.run().await, v_clone.run().await);

    // Homomorphism: pure f <*> pure x ≡ pure (f x)
    let x = 42;
    let mut left = Effect::<i32, std::convert::Infallible>::pure(x).ap(Effect::pure(double));
    let mut right = Effect::pure(double(x));
    assert_eq!(left.run().await, right.run().await);

    // Interchange: u <*> pure y ≡ pure ($ y) <*> u
    let u: Infallible<fn(i32) -> i32> = Effect::pure(double);
    let y = 42;
    let mut left = Effect::pure(y).ap(u);
    let u2: Infallible<fn(i32) -> i32> = Effect::pure(double);
    let mut right = u2.ap(Effect::pure(apply_to(y)));
    assert_eq!(left.run().await, right.run().await);
  }

  #[tokio::test]
  async fn test_effect_complex_composition() {
    let effect: Infallible<i32> = Effect::pure(42);
    let mut result = effect
      .map(double)
      .bind(|x| Effect::pure(x + 1))
      .ap(Effect::pure(triple));
    assert_eq!(result.run().await, Ok(255));
  }

  #[tokio::test]
  async fn test_effect_error_handling() {
    // Test error propagation
    let mut effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    assert_eq!(
      effect.run().await.unwrap_err().kind(),
      std::io::ErrorKind::Other
    );

    // Test error recovery
    let effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut recovered = effect.handle_error(|_| Effect::pure(42));
    assert_eq!(recovered.run().await.unwrap(), 42);

    // Test error mapping
    let effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut mapped =
      effect.map_error(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Error: {}", e)));
    assert_eq!(
      mapped.run().await.unwrap_err().kind(),
      std::io::ErrorKind::Other
    );

    // Test error chaining
    let effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut chained: Effect<i32, std::io::Error> = effect
      .and_then(|_| Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "new error")));
    assert_eq!(
      chained.run().await.unwrap_err().kind(),
      std::io::ErrorKind::Other
    );
  }

  #[tokio::test]
  async fn test_effect_error_recovery() {
    // Test error recovery with fallback
    let effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut recovered = effect.or_else(|_| Effect::pure(42));
    assert_eq!(recovered.run().await.unwrap(), 42);

    // Test error recovery with mapping
    let effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut recovered =
      effect.map_error(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Error: {}", e)));
    assert_eq!(
      recovered.run().await.unwrap_err().kind(),
      std::io::ErrorKind::Other
    );

    // Test error recovery with chaining
    let effect: Effect<i32, std::io::Error> =
      Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut chained: Effect<i32, std::io::Error> = effect
      .and_then(|_| Effect::error(std::io::Error::new(std::io::ErrorKind::Other, "new error")));
    assert_eq!(
      chained.run().await.unwrap_err().kind(),
      std::io::ErrorKind::Other
    );
  }

  #[tokio::test]
  async fn test_effect_infallible() {
    // Test infallible effect
    let mut effect: Infallible<i32> = Effect::pure(42);
    assert_eq!(effect.run().await, Ok(42));

    // Test infallible effect with mapping
    let effect: Infallible<i32> = Effect::pure(42);
    let mut mapped = effect.map(|x| x * 2);
    assert_eq!(mapped.run().await, Ok(84));
  }

  #[tokio::test]
  async fn test_effect_any_error() {
    // Test any error effect
    let mut effect: AnyError<i32> = Effect::pure(42);
    assert!(effect.run().await.is_ok());

    // Test any error effect with error
    let err: Box<dyn std::error::Error + Send + Sync> =
      Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
    let mut effect: AnyError<i32> = Effect::error(err);
    assert!(effect.run().await.is_err());
  }
}
