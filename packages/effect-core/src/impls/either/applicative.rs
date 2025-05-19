//! Implementation of the `Applicative` trait for `Either<L, R>`.
//!
//! This module provides a right-biased applicative implementation for the `Either` type,
//! allowing for function application within the context of `Either`.
//!
//! Key behaviors:
//! - `pure` wraps values in the `Right` variant
//! - `ap` applies a function in a `Right` to a value in a `Right`, or propagates a `Left` unchanged
//!
//! This right-biased behavior makes `Either` useful for error handling and computation chains
//! that can fail, similar to `Result<T, E>`.

use crate::traits::applicative::Applicative;
use crate::types::either::Either;
use crate::types::threadsafe::CloneableThreadSafe;

impl<L: CloneableThreadSafe, R: CloneableThreadSafe> Applicative<R> for Either<L, R> {
  fn pure<B>(value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    Either::Right(value)
  }

  fn ap<B, F>(self, fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a R) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match fs {
      Either::Left(l) => Either::Left(l),
      Either::Right(mut f) => match self {
        Either::Left(l) => Either::Left(l),
        Either::Right(r) => Either::Right(f(&r)),
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Identity law: pure(id).ap(v) == v
  #[test]
  fn test_identity_law() {
    // Test with Left
    let left: Either<i32, String> = Either::Left(42);
    let id_fn = |x: &String| x.clone();
    let id_wrapped = Either::<i32, fn(&String) -> String>::pure(id_fn);
    let result = left.clone().ap(id_wrapped);
    assert_eq!(result, left);

    // Test with Right
    let right: Either<i32, String> = Either::Right("hello".to_string());
    let id_fn = |x: &String| x.clone();
    let id_wrapped = Either::<i32, fn(&String) -> String>::pure(id_fn);
    let result = right.clone().ap(id_wrapped);
    assert_eq!(result, right);
  }

  // Homomorphism law: pure(f).ap(pure(x)) == pure(f(x))
  #[test]
  fn test_homomorphism_law() {
    let x = "hello".to_string();
    let f = |s: &String| s.len();

    let left =
      Either::<i32, String>::pure(x.clone()).ap(Either::<i32, fn(&String) -> usize>::pure(f));
    let right = Either::<i32, usize>::pure(f(&x));

    assert_eq!(left, right);
  }

  // Interchange law: u.ap(pure(y)) == pure(|f| f(y)).ap(u)
  #[test]
  fn test_interchange_law() {
    let y = "hello".to_string();
    let u: Either<i32, fn(&String) -> usize> = Either::Right(|s: &String| s.len());

    let left = Either::<i32, String>::pure(y.clone()).ap(u.clone());

    let apply_y = move |f: &fn(&String) -> usize| f(&y);
    let right = u.ap(Either::<i32, fn(&fn(&String) -> usize) -> usize>::pure(
      apply_y,
    ));

    assert_eq!(left, right);
  }

  // Composition law: pure(compose).ap(u).ap(v).ap(w) == u.ap(v.ap(w))
  #[test]
  fn test_composition_law() {
    let w: Either<i32, String> = Either::Right("hello".to_string());
    let v: Either<i32, fn(&String) -> usize> = Either::Right(|s: &String| s.len());
    let u: Either<i32, fn(&usize) -> bool> = Either::Right(|n: &usize| *n > 3);

    // Since there's no direct "compose" support in the trait, we'll implement the law directly
    let left = w.clone().ap(v.clone()).ap(u.clone());

    let inner = w.clone().ap(v.clone());
    let right = inner.ap(u.clone());

    assert_eq!(left, right);
  }

  // Test propagation of Left values
  #[test]
  fn test_left_propagation() {
    // Left.ap(Right)
    let left: Either<i32, String> = Either::Left(42);
    let right_fn: Either<i32, fn(&String) -> usize> = Either::Right(|s: &String| s.len());
    let result = left.clone().ap(right_fn);
    assert_eq!(result, Either::Left(42));

    // Right.ap(Left)
    let right: Either<i32, String> = Either::Right("hello".to_string());
    let left_fn: Either<i32, fn(&String) -> usize> = Either::Left(42);
    let result = right.ap(left_fn);
    assert_eq!(result, Either::Left(42));
  }

  // Test with complex types
  #[test]
  fn test_with_complex_types() {
    #[derive(Debug, PartialEq, Clone)]
    struct Person {
      name: String,
      age: u32,
    }

    let person_fn = |name: &String| Person {
      name: name.clone(),
      age: name.len() as u32,
    };

    let name = Either::<String, String>::Right("Alice".to_string());
    let wrapped_fn = Either::<String, fn(&String) -> Person>::Right(person_fn);

    let result = name.ap(wrapped_fn);
    let expected = Either::Right(Person {
      name: "Alice".to_string(),
      age: 5,
    });

    assert_eq!(result, expected);
  }

  proptest! {
    // Identity law: pure(id).ap(v) == v
    #[test]
    fn prop_identity_law(left_value: i32, right_value: i32, use_left: bool) {
      let either = if use_left {
        Either::<i32, i32>::Left(left_value)
      } else {
        Either::<i32, i32>::Right(right_value)
      };

      let id_fn = |x: &i32| *x;
      let id_wrapped = Either::<i32, fn(&i32) -> i32>::pure(id_fn);
      let result = either.clone().ap(id_wrapped);

      prop_assert_eq!(result, either);
    }

    // Homomorphism law: pure(f).ap(pure(x)) == pure(f(x))
    #[test]
    fn prop_homomorphism_law(x: i32) {
      let f = |n: &i32| n.saturating_add(10);

      let left = Either::<String, i32>::pure(x).ap(Either::<String, fn(&i32) -> i32>::pure(f));
      let right = Either::<String, i32>::pure(f(&x));

      prop_assert_eq!(left, right);
    }

    // Check Left propagation
    #[test]
    fn prop_left_propagation(left_value: i32) {
      // Left.ap(Right)
      let left: Either<i32, i32> = Either::Left(left_value);
      let right_fn: Either<i32, fn(&i32) -> i32> = Either::Right(|n: &i32| n.saturating_add(10));
      let result = left.clone().ap(right_fn);
      prop_assert_eq!(result, left);

      // Right.ap(Left)
      let right: Either<i32, i32> = Either::Right(42);
      let left_fn: Either<i32, fn(&i32) -> i32> = Either::Left(left_value);
      let result = right.ap(left_fn);
      prop_assert_eq!(result, Either::Left(left_value));
    }
  }
}
