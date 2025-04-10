use crate::functor::Functor;
use crate::monad::Monad;
use std::fmt::{Debug, Display};

/// A type that represents an optional value.
/// Similar to Rust's built-in Option but with Effect system integration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Option<T> {
  None,
  Some(T),
}

impl<T> Option<T> {
  /// Creates a new Some value.
  pub fn some(value: T) -> Self {
    Self::Some(value)
  }

  /// Creates a new None value.
  pub fn none() -> Self {
    Self::None
  }

  /// Returns true if the option is a Some value.
  pub fn is_some(&self) -> bool {
    matches!(self, Self::Some(_))
  }

  /// Returns true if the option is a None value.
  pub fn is_none(&self) -> bool {
    matches!(self, Self::None)
  }

  /// Unwraps the option, yielding the content of a Some.
  pub fn unwrap(self) -> T {
    match self {
      Self::Some(value) => value,
      Self::None => panic!("called `Option::unwrap()` on a `None` value"),
    }
  }

  /// Returns the contained Some value or a default.
  pub fn unwrap_or(self, default: T) -> T {
    match self {
      Self::Some(value) => value,
      Self::None => default,
    }
  }

  /// Maps an Option<T> to Option<U> by applying a function to a contained value.
  pub fn map<U, F>(self, f: F) -> Option<U>
  where
    F: FnOnce(T) -> U,
  {
    match self {
      Self::Some(value) => Option::Some(f(value)),
      Self::None => Option::None,
    }
  }

  /// Applies a function to the contained value (if any), or returns the provided default (if not).
  pub fn map_or<U, F>(self, default: U, f: F) -> U
  where
    F: FnOnce(T) -> U,
  {
    match self {
      Self::Some(value) => f(value),
      Self::None => default,
    }
  }
}

impl<T: Send + Sync + 'static> Functor<T> for Option<T> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match self {
      Self::Some(value) => Option::Some(f(value)),
      Self::None => Option::None,
    }
  }
}

impl<T: Send + Sync + 'static> Monad<T> for Option<T> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn pure(a: T) -> Self::HigherSelf<T> {
    Option::Some(a)
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    match self {
      Self::Some(value) => f(value),
      Self::None => Option::None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_option_creation() {
    let some = Option::some(42);
    assert!(some.is_some());
    assert_eq!(some.unwrap(), 42);

    let none = Option::<i32>::none();
    assert!(none.is_none());
  }

  #[test]
  fn test_option_map() {
    let some = Option::some(42);
    let mapped = some.map(|x| x * 2);
    assert_eq!(mapped.unwrap(), 84);

    let none = Option::<i32>::none();
    let mapped = none.map(|x| x * 2);
    assert!(mapped.is_none());
  }

  #[test]
  fn test_option_map_or() {
    let some = Option::some(42);
    let result = some.map_or(0, |x| x * 2);
    assert_eq!(result, 84);

    let none = Option::<i32>::none();
    let result = none.map_or(0, |x| x * 2);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_option_unwrap_or() {
    let some = Option::some(42);
    assert_eq!(some.unwrap_or(0), 42);

    let none = Option::<i32>::none();
    assert_eq!(none.unwrap_or(0), 0);
  }

  #[test]
  fn test_functor_laws() {
    // Identity
    let some = Option::some(42);
    let some_clone = some.clone();
    let mapped = some.map(|x| x);
    assert_eq!(some_clone, mapped);

    // Composition
    let f = |x: i32| x * 2;
    let g = |x: i32| x + 1;
    let some = Option::some(42);
    let some_clone = some.clone();
    let mapped1 = some.map(|x| g(f(x)));
    let mapped2 = some_clone.map(f).map(g);
    assert_eq!(mapped1, mapped2);
  }

  #[test]
  fn test_monad_laws() {
    // Left identity
    let a = 42;
    let f = |x: i32| Option::some(x * 2);
    assert_eq!(Option::pure(a).bind(f), f(a));

    // Right identity
    let m = Option::some(42);
    let m_clone = m.clone();
    assert_eq!(m.bind(Option::pure), m_clone);

    // Associativity
    let m = Option::some(42);
    let m_clone = m.clone();
    let f = |x: i32| Option::some(x * 2);
    let g = |x: i32| Option::some(x + 1);
    let f_clone = f;
    let g_clone = g;
    assert_eq!(
      m.bind(move |x| f(x).bind(g_clone)),
      m_clone.bind(f_clone).bind(g)
    );
  }
}
