//! Either type implementation for error handling.
//!
//! This module provides the `Either` type, which represents a value that can be
//! either a left value or a right value. It is commonly used for error handling
//! in the effect system.

use std::fmt;

/// A type that represents either a left value or a right value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Either<L, R> {
  /// The left variant.
  Left(L),
  /// The right variant.
  Right(R),
}

impl<L, R> Either<L, R> {
  /// Creates a new `Either` with a left value.
  pub fn left(value: L) -> Self {
    Self::Left(value)
  }

  /// Creates a new `Either` with a right value.
  pub fn right(value: R) -> Self {
    Self::Right(value)
  }

  /// Returns `true` if this is a left value.
  pub fn is_left(&self) -> bool {
    matches!(self, Self::Left(_))
  }

  /// Returns `true` if this is a right value.
  pub fn is_right(&self) -> bool {
    matches!(self, Self::Right(_))
  }

  /// Maps the left value using the given function.
  pub fn map_left<F, T>(self, f: F) -> Either<T, R>
  where
    F: FnOnce(L) -> T,
  {
    match self {
      Self::Left(l) => Either::Left(f(l)),
      Self::Right(r) => Either::Right(r),
    }
  }

  /// Maps the right value using the given function.
  pub fn map_right<F, T>(self, f: F) -> Either<L, T>
  where
    F: FnOnce(R) -> T,
  {
    match self {
      Self::Left(l) => Either::Left(l),
      Self::Right(r) => Either::Right(f(r)),
    }
  }

  /// Unwraps the left value, panicking if this is a right value.
  pub fn unwrap_left(self) -> L {
    match self {
      Self::Left(l) => l,
      Self::Right(_) => panic!("called `unwrap_left` on a `Right` value"),
    }
  }

  /// Unwraps the right value, panicking if this is a left value.
  pub fn unwrap_right(self) -> R {
    match self {
      Self::Left(_) => panic!("called `unwrap_right` on a `Left` value"),
      Self::Right(r) => r,
    }
  }
}

impl<L, R> fmt::Display for Either<L, R>
where
  L: fmt::Display,
  R: fmt::Display,
{
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Left(l) => write!(f, "Left({})", l),
      Self::Right(r) => write!(f, "Right({})", r),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Basic construction tests
  #[test]
  fn test_construction() {
    let left = Either::left(42);
    assert!(left.is_left());
    assert!(!left.is_right());
    assert_eq!(left.unwrap_left(), 42);

    let right = Either::right("hello");
    assert!(!right.is_left());
    assert!(right.is_right());
    assert_eq!(right.unwrap_right(), "hello");
  }

  // Map tests
  #[test]
  fn test_map_left() {
    let left = Either::left(42);
    let mapped = left.map_left(|x| x * 2);
    assert_eq!(mapped.unwrap_left(), 84);

    let right = Either::right("hello");
    let mapped = right.map_left(|x: i32| x * 2);
    assert_eq!(mapped.unwrap_right(), "hello");
  }

  #[test]
  fn test_map_right() {
    let left = Either::left(42);
    let mapped = left.map_right(|s: &str| s.len());
    assert_eq!(mapped.unwrap_left(), 42);

    let right = Either::right("hello");
    let mapped = right.map_right(|s| s.len());
    assert_eq!(mapped.unwrap_right(), 5);
  }

  // Unwrap tests
  #[test]
  #[should_panic(expected = "called `unwrap_left` on a `Right` value")]
  fn test_unwrap_left_panic() {
    let right = Either::right("hello");
    right.unwrap_left();
  }

  #[test]
  #[should_panic(expected = "called `unwrap_right` on a `Left` value")]
  fn test_unwrap_right_panic() {
    let left = Either::left(42);
    left.unwrap_right();
  }

  // Display tests
  #[test]
  fn test_display() {
    let left = Either::left(42);
    assert_eq!(format!("{}", left), "Left(42)");

    let right = Either::right("hello");
    assert_eq!(format!("{}", right), "Right(hello)");
  }

  // Clone tests
  #[test]
  fn test_clone() {
    let left = Either::left(42);
    let cloned = left.clone();
    assert_eq!(left, cloned);

    let right = Either::right("hello");
    let cloned = right.clone();
    assert_eq!(right, cloned);
  }

  // Debug tests
  #[test]
  fn test_debug() {
    let left = Either::left(42);
    assert_eq!(format!("{:?}", left), "Left(42)");

    let right = Either::right("hello");
    assert_eq!(format!("{:?}", right), "Right(\"hello\")");
  }

  // PartialEq tests
  #[test]
  fn test_partial_eq() {
    let left1 = Either::left(42);
    let left2 = Either::left(42);
    let left3 = Either::left(43);
    assert_eq!(left1, left2);
    assert_ne!(left1, left3);

    let right1 = Either::right("hello");
    let right2 = Either::right("hello");
    let right3 = Either::right("world");
    assert_eq!(right1, right2);
    assert_ne!(right1, right3);

    assert_ne!(left1, right1);
  }

  // Complex type tests
  #[test]
  fn test_complex_types() {
    let left = Either::left(vec![1, 2, 3]);
    let mapped = left.map_left(|v| v.into_iter().sum::<i32>());
    assert_eq!(mapped.unwrap_left(), 6);

    let right = Either::right(Some("hello"));
    let mapped = right.map_right(|opt| opt.map(|s| s.len()));
    assert_eq!(mapped.unwrap_right(), Some(5));
  }

  // Nested Either tests
  #[test]
  fn test_nested_either() {
    let left = Either::left(Either::left(42));
    let mapped = left.map_left(|e| e.map_left(|x| x * 2));
    assert_eq!(mapped.unwrap_left().unwrap_left(), 84);

    let right = Either::right(Either::right("hello"));
    let mapped = right.map_right(|e| e.map_right(|s| s.len()));
    assert_eq!(mapped.unwrap_right().unwrap_right(), 5);
  }

  // Zero-sized type tests
  #[test]
  fn test_zero_sized_types() {
    let left = Either::left(());
    assert!(left.is_left());
    assert_eq!(left.unwrap_left(), ());

    let right = Either::right(());
    assert!(right.is_right());
    assert_eq!(right.unwrap_right(), ());
  }

  // Reference type tests
  #[test]
  fn test_reference_types() {
    let value = 42;
    let left = Either::left(&value);
    assert_eq!(*left.unwrap_left(), 42);

    let string = "hello".to_string();
    let right = Either::right(&string);
    assert_eq!(*right.unwrap_right(), "hello");
  }
}
