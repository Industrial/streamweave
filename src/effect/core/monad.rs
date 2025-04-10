//! Monad trait and implementations for the Effect type.
//!
//! This module defines the `Monad` trait and provides implementations for the
//! `Effect` type, enabling monadic composition and transformation of effects.

use std::{cmp::PartialEq, convert::From, fmt::Debug};

/// The Monad trait defines the basic operations for monadic types.
pub trait Monad {
  /// The inner type of the monad.
  type Inner: Debug + PartialEq + Send + Sync + 'static;
  /// The type of the monad after mapping.
  type Mapped<U>: Monad<Inner = U>
  where
    U: Debug + PartialEq + Send + Sync + 'static;

  /// Creates a new monad from a value.
  fn pure<T>(value: T) -> Self
  where
    Self: Sized,
    T: Debug + PartialEq + Send + Sync + 'static,
    Self::Inner: From<T>;

  /// Transforms the inner value of the monad.
  fn map<F, U>(self, f: F) -> Self::Mapped<U>
  where
    F: FnOnce(Self::Inner) -> U + Send + Sync + 'static,
    U: Debug + PartialEq + Send + Sync + 'static,
    Self: Sized;

  /// Composes two monads, applying a function to the inner value.
  fn flat_map<F>(self, f: F) -> Self
  where
    F: FnOnce(Self::Inner) -> Self + Send + Sync + 'static,
    Self: Sized;
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
  };

  // Define a simple monad for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestMonad<T>(T);

  impl<T: Debug + PartialEq + Send + Sync + 'static> TestMonad<T> {
    fn new(value: T) -> Self {
      TestMonad(value)
    }

    // Helper method to create a monad with a specific type
    fn of<U>(value: U) -> TestMonad<U>
    where
      U: Debug + PartialEq + Send + Sync + 'static,
    {
      TestMonad(value)
    }
  }

  impl<T: Debug + PartialEq + Send + Sync + 'static> Monad for TestMonad<T> {
    type Inner = T;
    type Mapped<U>
      = TestMonad<U>
    where
      U: Debug + PartialEq + Send + Sync + 'static;

    fn pure<U>(value: U) -> Self
    where
      U: Debug + PartialEq + Send + Sync + 'static,
      T: From<U>,
    {
      TestMonad(value.into())
    }

    fn map<F, U>(self, f: F) -> Self::Mapped<U>
    where
      F: FnOnce(T) -> U + Send + Sync + 'static,
      U: Debug + PartialEq + Send + Sync + 'static,
    {
      TestMonad(f(self.0))
    }

    fn flat_map<F>(self, f: F) -> Self
    where
      F: FnOnce(T) -> Self + Send + Sync + 'static,
    {
      f(self.0)
    }
  }

  // Test monad laws
  #[test]
  fn test_monad_laws() {
    // Left identity: pure(a).flat_map(f) == f(a)
    let a = 42;
    let f = |x: i32| TestMonad::new(x * 2);
    assert_eq!(TestMonad::pure(a).flat_map(f), f(a));

    // Right identity: m.flat_map(pure) == m
    let m = TestMonad::new(42);
    assert_eq!(m.clone().flat_map(TestMonad::pure), m);

    // Associativity: m.flat_map(f).flat_map(g) == m.flat_map(|x| f(x).flat_map(g))
    let m = TestMonad::new(42);
    let f = |x: i32| TestMonad::new(x * 2);
    let g = |x: i32| TestMonad::new(x + 1);
    assert_eq!(
      m.clone().flat_map(f).flat_map(g),
      m.flat_map(move |x| f(x).flat_map(g))
    );
  }

  // Test basic operations
  #[test]
  fn test_pure() {
    let m: TestMonad<i32> = TestMonad::pure(42);
    assert_eq!(m, TestMonad(42));
  }

  #[test]
  fn test_map() {
    let m: TestMonad<i32> = TestMonad::pure(42).map(|x: i32| x * 2);
    assert_eq!(m, TestMonad(84));
  }

  #[test]
  fn test_flat_map() {
    let m: TestMonad<i32> = TestMonad::pure(42).flat_map(|x| TestMonad::new(x * 2));
    assert_eq!(m, TestMonad(84));
  }

  // Test type conversions
  #[test]
  fn test_type_conversions() {
    let m: TestMonad<String> = TestMonad::of(42).map(|x: i32| x.to_string());
    assert_eq!(m, TestMonad("42".to_string()));
  }

  // Test complex composition
  #[test]
  fn test_complex_composition() {
    let m: TestMonad<String> = TestMonad::of(1)
      .flat_map(|x: i32| TestMonad::of(x + 1))
      .map(|x: i32| x * 2)
      .flat_map(|x: i32| TestMonad::of(x + 1))
      .map(|x: i32| x.to_string());

    assert_eq!(m, TestMonad("5".to_string()));
  }

  // Test nested composition
  #[test]
  fn test_nested_composition() {
    let m: TestMonad<i32> = TestMonad::of(1).flat_map(move |x: i32| {
      TestMonad::of(x + 1).flat_map(move |y: i32| TestMonad::of(y * 2).map(move |z: i32| z + x))
    });

    assert_eq!(m, TestMonad(5));
  }
}
