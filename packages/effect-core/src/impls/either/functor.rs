//! Implementation of the `Functor` trait for `Either<L, R>`.
//!
//! This module provides a right-biased functor implementation for the `Either` type,
//! meaning that `map` applies the function only to the `Right` variant, leaving `Left` values unchanged.
//! This is similar to how `Result<T, E>` maps only on the `Ok` variant.
//!
//! This right-biased behavior is useful for error handling, where `Left` represents errors or failures,
//! and `Right` represents successful values that can be further processed.

use crate::traits::functor::Functor;
use crate::types::either::Either;
use crate::types::threadsafe::CloneableThreadSafe;

impl<L: CloneableThreadSafe, R: CloneableThreadSafe> Functor<R> for Either<L, R> {
  type HigherSelf<U: CloneableThreadSafe> = Either<L, U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a R) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    match self {
      Either::Left(l) => Either::Left(l),
      Either::Right(r) => Either::Right(f(&r)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_identity_law() {
    // Test with Left
    let left: Either<i32, String> = Either::Left(42);
    let mapped = left.clone().map(|x: &String| x.clone());
    assert_eq!(left, mapped);

    // Test with Right
    let right: Either<i32, String> = Either::Right("hello".to_string());
    let mapped = right.clone().map(|x: &String| x.clone());
    assert_eq!(right, mapped);
  }

  #[test]
  fn test_composition_law() {
    // Test with Left
    let left: Either<i32, String> = Either::Left(42);
    let f = |x: &String| x.len();
    let g = |x: &usize| x * 2;

    let mapped1 = left.clone().map(f).map(g);
    let mapped2 = left.clone().map(move |x| g(&f(x)));
    assert_eq!(mapped1, mapped2);

    // Test with Right
    let right: Either<i32, String> = Either::Right("hello".to_string());
    let mapped1 = right.clone().map(f).map(g);
    let mapped2 = right.clone().map(move |x| g(&f(x)));
    assert_eq!(mapped1, mapped2);
  }

  #[test]
  fn test_map_preserves_left() {
    let left: Either<i32, String> = Either::Left(42);
    let mapped = left.map(|x: &String| x.len());

    assert_eq!(mapped, Either::Left(42));
  }

  #[test]
  fn test_map_transforms_right() {
    let right: Either<i32, String> = Either::Right("hello".to_string());
    let mapped = right.map(|x: &String| x.len());

    assert_eq!(mapped, Either::Right(5));
  }

  proptest! {
    #[test]
    fn prop_identity_law(left_value: i32, right_value: i32, use_left: bool) {
      let either = if use_left {
        Either::Left(left_value)
      } else {
        Either::Right(right_value)
      };

      let mapped = either.clone().map(|x: &i32| *x);
      prop_assert_eq!(either, mapped);
    }

    #[test]
    fn prop_composition_law(left_value: i32, right_value: i32, use_left: bool) {
      let either = if use_left {
        Either::Left(left_value)
      } else {
        Either::Right(right_value)
      };

      let f = |x: &i32| x.saturating_add(1);
      let g = |x: &i32| x.saturating_mul(2);

      let mapped1 = either.clone().map(f).map(g);
      let mapped2 = either.clone().map(move |x| g(&f(x)));

      prop_assert_eq!(mapped1, mapped2);
    }

    #[test]
    fn prop_map_preserves_left(left_value: i32) {
      let left: Either<i32, i32> = Either::Left(left_value);
      let mapped = left.map(|x: &i32| x.saturating_add(10));

      prop_assert_eq!(mapped, Either::Left(left_value));
    }

    #[test]
    fn prop_map_transforms_right(right_value: i32) {
      let right: Either<i32, i32> = Either::Right(right_value);
      let mapped = right.map(|x: &i32| x.saturating_add(10));

      prop_assert_eq!(mapped, Either::Right(right_value.saturating_add(10)));
    }
  }
}
