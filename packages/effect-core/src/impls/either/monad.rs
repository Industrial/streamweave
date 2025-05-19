//! Implementation of the `Monad` trait for `Either<L, R>`.
//!
//! This module provides a right-biased monad implementation for the `Either` type,
//! which enables sequential, chainable operations that can fail or short-circuit.
//!
//! Key behaviors:
//! - `bind` applies a function to the `Right` variant or propagates the `Left` variant unchanged
//! - `Left` values short-circuit computations, similar to how `Err` values do in `Result`
//!
//! This makes `Either` useful for representing computations that can fail with different error types,
//! or for railway-oriented programming patterns.

use crate::traits::monad::Monad;
use crate::types::either::Either;
use crate::types::threadsafe::CloneableThreadSafe;

impl<L: CloneableThreadSafe, R: CloneableThreadSafe> Monad<R> for Either<L, R> {
  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a R) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match self {
      Either::Left(l) => Either::Left(l),
      Either::Right(r) => f(&r),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::applicative::Applicative;
  use proptest::prelude::*;

  // Helper functions for testing
  fn safe_divide(x: &i32, y: &i32) -> Either<String, i32> {
    if *y == 0 {
      Either::Left("Division by zero".to_string())
    } else {
      Either::Right(x / y)
    }
  }

  fn add_one(x: &i32) -> Either<String, i32> {
    Either::Right(x + 1)
  }

  fn double(x: &i32) -> Either<String, i32> {
    Either::Right(x * 2)
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i32;
    let f = |x: &i32| -> Either<String, i32> { Either::Right(x + 10) };

    let left = Either::<String, i32>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(left, right);
    assert_eq!(left, Either::Right(52));
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    // Test with Right value
    let m = Either::<String, i32>::Right(42);
    let pure_fn = |x: &i32| Either::<String, i32>::pure(*x);

    let result = m.clone().bind(pure_fn);
    assert_eq!(result, m);

    // Test with Left value
    let m = Either::<String, i32>::Left("error".to_string());
    let result = m.clone().bind(pure_fn);
    assert_eq!(result, m);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m = Either::<String, i32>::Right(5);
    let f = |x: &i32| -> Either<String, i32> { Either::Right(x + 1) };
    let g = |x: &i32| -> Either<String, i32> { Either::Right(x * 2) };

    let left = m.clone().bind(f).bind(g);

    let right = m.bind(move |x| f(x).bind(g));

    assert_eq!(left, right);
    assert_eq!(left, Either::Right(12)); // (5+1)*2

    // Also test with a Left value
    let m = Either::<String, i32>::Left("error".to_string());

    let left = m.clone().bind(f).bind(g);
    let right = m.bind(move |x| f(x).bind(g));

    assert_eq!(left, right);
    assert_eq!(left, Either::Left("error".to_string()));
  }

  // Test with Left propagation
  #[test]
  fn test_left_propagation() {
    let m = Either::<String, i32>::Left("error".to_string());
    let f = |x: &i32| -> Either<String, i32> { Either::Right(x + 1) };

    let result = m.clone().bind(f);
    assert_eq!(result, m);
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    // Start with a Right value
    let m = Either::<String, i32>::Right(10);

    // Chain operations that should succeed
    let result = m.clone().bind(|x| add_one(x)).bind(|x| double(x));
    assert_eq!(result, Either::Right(22)); // (10+1)*2

    // Chain with an operation that will fail
    let result = m.bind(|x| add_one(x)).bind(|x| safe_divide(x, &0));
    assert_eq!(result, Either::Left("Division by zero".to_string()));
  }

  // Test with conditional logic in bind
  #[test]
  fn test_conditional_bind() {
    let m = Either::<String, i32>::Right(10);

    // Use bind with conditional logic
    let result = m.bind(|x| {
      if *x > 5 {
        Either::Right(*x * 2)
      } else {
        Either::Left("Value too small".to_string())
      }
    });

    assert_eq!(result, Either::Right(20));

    // Test with a value that will trigger the Left condition
    let m = Either::<String, i32>::Right(3);
    let result = m.bind(|x| {
      if *x > 5 {
        Either::Right(*x * 2)
      } else {
        Either::Left("Value too small".to_string())
      }
    });

    assert_eq!(result, Either::Left("Value too small".to_string()));
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m = Either::<String, i32>::Right(10);

    // flat_map should behave the same as bind
    let bind_result = m.clone().bind(|x| add_one(x));
    let flat_map_result = m.flat_map(|x| add_one(x));

    assert_eq!(bind_result, flat_map_result);
  }

  // Property-based tests
  proptest! {
    // Left identity law
    #[test]
    fn prop_left_identity(a in -100..100i32) {
      let f = |x: &i32| -> Either<String, i32> { Either::Right(x.saturating_add(10)) };

      let left = Either::<String, i32>::pure(a).bind(f);
      let right = f(&a);

      prop_assert_eq!(left, right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(x in -100..100i32, is_left in prop::bool::ANY) {
      let m = if is_left {
        Either::<String, i32>::Left("error".to_string())
      } else {
        Either::<String, i32>::Right(x)
      };

      let pure_fn = |y: &i32| Either::<String, i32>::pure(*y);
      let result = m.clone().bind(pure_fn);

      prop_assert_eq!(result, m);
    }

    // Associativity law
    #[test]
    fn prop_associativity(x in -100..100i32, is_left in prop::bool::ANY) {
      let m = if is_left {
        Either::<String, i32>::Left("error".to_string())
      } else {
        Either::<String, i32>::Right(x)
      };

      let f = |y: &i32| -> Either<String, i32> { Either::Right(y.saturating_add(1)) };
      let g = |y: &i32| -> Either<String, i32> { Either::Right(y.saturating_mul(2)) };

      let left = m.clone().bind(f).bind(g);
      let right = m.bind(move |y| f(y).bind(g));

      prop_assert_eq!(left, right);
    }

    // Test Left propagation
    #[test]
    fn prop_left_propagation(error_msg in "[a-zA-Z0-9 ]+") {
      let m = Either::<String, i32>::Left(error_msg.clone());
      let f = |x: &i32| -> Either<String, i32> { Either::Right(x.saturating_add(1)) };

      let result = m.clone().bind(f);
      prop_assert_eq!(result, m);
    }

    // Test flat_map equivalence
    #[test]
    fn prop_flat_map_equiv_bind(x in -100..100i32, is_left in prop::bool::ANY) {
      let m = if is_left {
        Either::<String, i32>::Left("error".to_string())
      } else {
        Either::<String, i32>::Right(x)
      };

      let f = |y: &i32| -> Either<String, i32> { Either::Right(y.saturating_add(10)) };

      let bind_result = m.clone().bind(f);
      let flat_map_result = m.flat_map(f);

      prop_assert_eq!(bind_result, flat_map_result);
    }
  }
}
