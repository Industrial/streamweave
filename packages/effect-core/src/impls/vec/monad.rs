//! Implementation of the `Monad` trait for `Vec<A>`.
//!
//! This module implements the monadic bind operation for the `Vec` type,
//! enabling non-deterministic computation where each element in the vector
//! can produce multiple results.
//!
//! The implementation flattens a nested vector, representing the application
//! of a function that returns multiple possibilities to a collection of
//! values. This is similar to the concept of the "list monad" in Haskell,
//! where lists represent non-deterministic computations with multiple potential outcomes.

use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Monad<A> for Vec<A> {
  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = Vec::new();
    for a in self {
      result.extend(f(&a));
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::applicative::Applicative;
  use proptest::prelude::*;

  // Test functions for the monad laws
  fn add_one_to_many(x: &i64) -> Vec<i64> {
    vec![x + 1, x + 2]
  }

  fn multiply_by(x: &i64) -> Vec<i64> {
    vec![x * 2, x * 3]
  }

  // Function that returns an empty Vec for some inputs
  fn filter_positive(x: &i64) -> Vec<i64> {
    if *x > 0 {
      vec![*x]
    } else {
      vec![]
    }
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i64;
    let f = add_one_to_many;

    let left = Vec::<i64>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(left, right);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let m = vec![1, 2, 3];

    let left = m.clone().bind(|x| Vec::<i64>::pure(*x));
    let right = m;

    assert_eq!(left, right);

    // Also test with empty vector
    let empty: Vec<i64> = Vec::new();
    let left = empty.clone().bind(|x| Vec::<i64>::pure(*x));

    assert_eq!(left, empty);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m = vec![5, 10];
    let f = add_one_to_many;
    let g = multiply_by;

    let left = m.clone().bind(f).bind(g);

    let right = m.bind(|x| {
      let f_result = add_one_to_many(x);
      f_result.bind(multiply_by)
    });

    assert_eq!(left, right);

    // Also test with empty vector
    let empty: Vec<i64> = Vec::new();
    let left = empty.clone().bind(f).bind(g);

    let right = empty.bind(|x| {
      let f_result = add_one_to_many(x);
      f_result.bind(multiply_by)
    });

    assert_eq!(left, right);
  }

  // Test with empty vector
  #[test]
  fn test_with_empty() {
    let empty: Vec<i64> = Vec::new();
    let f = add_one_to_many;

    assert_eq!(empty.bind(f), Vec::new());
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let m = vec![1, 2];

    // 1 -> [2, 3], 2 -> [3, 4], then each multiplied by 2 and 3
    let result = m.bind(add_one_to_many).bind(multiply_by);
    let expected = vec![
      // From 1 -> 2 -> 4, 6
      4, 6, // From 1 -> 3 -> 6, 9
      6, 9, // From 2 -> 3 -> 6, 9
      6, 9, // From 2 -> 4 -> 8, 12
      8, 12,
    ];
    assert_eq!(result, expected);
  }

  // Test with functions that can return empty vectors (filtering)
  #[test]
  fn test_with_filtering_functions() {
    let m = vec![-2, 0, 3, 5];

    // Should filter out negative and zero values
    let result = m.clone().bind(filter_positive);
    assert_eq!(result, vec![3, 5]);

    // Chain with another operation
    let result = m.bind(filter_positive).bind(multiply_by);
    assert_eq!(result, vec![6, 9, 10, 15]);
  }

  // Test combining bind with then
  #[test]
  fn test_bind_then() {
    let m = vec![1, 2];
    let n = vec![10, 20];

    // Calculate n.bind(multiply_by) once
    let n_multiplied = n.clone().bind(multiply_by);
    // This should be vec![20, 30, 40, 60]

    // m.bind(add_one_to_many) produces [2, 3, 3, 4]
    // then(n_multiplied) discards the left results and repeats the right results
    // for each element in the left collection
    let result = m.clone().bind(add_one_to_many).then(n_multiplied.clone());

    // For each of the 4 elements in m.bind(add_one_to_many), we get all elements of n_multiplied
    let expected = vec![
      // For the first element from add_one_to_many (2)
      20, 30, 40, 60, // For the second element from add_one_to_many (3)
      20, 30, 40, 60, // For the third element from add_one_to_many (3)
      20, 30, 40, 60, // For the fourth element from add_one_to_many (4)
      20, 30, 40, 60,
    ];
    assert_eq!(result, expected);

    // If left side is empty, then should produce empty
    let empty: Vec<i64> = Vec::new();
    let result = empty.then(n_multiplied);
    assert_eq!(result, Vec::new());

    // If right side is empty, then should produce empty
    let result = m.then(Vec::<i64>::new());
    assert_eq!(result, Vec::new());
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m = vec![1, 2];

    // flat_map should behave the same as bind
    let bind_result = m.clone().bind(add_one_to_many);
    let flat_map_result = m.flat_map(add_one_to_many);

    assert_eq!(bind_result, flat_map_result);
  }

  // Property-based tests
  proptest! {
    // Left identity law
    #[test]
    fn prop_left_identity(a in -100..100i64) {
      let left = Vec::<i64>::pure(a).bind(add_one_to_many);
      let right = add_one_to_many(&a);

      prop_assert_eq!(left, right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let m = xs.clone();

      let left = m.clone().bind(|x| Vec::<i64>::pure(*x));
      let right = m;

      prop_assert_eq!(left, right);
    }

    // Associativity law
    #[test]
    fn prop_associativity(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let m = xs;

      let f = add_one_to_many;
      let g = multiply_by;

      let left = m.clone().bind(f).bind(g);

      let right = m.bind(|x| {
        let f_result = add_one_to_many(x);
        f_result.bind(multiply_by)
      });

      prop_assert_eq!(left, right);
    }

    // Test that bind with a function returning an empty Vec for all inputs produces an empty Vec
    #[test]
    fn prop_empty_result(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let m = xs;
      let empty_fn = |_: &i64| Vec::<i64>::new();

      prop_assert_eq!(m.bind(empty_fn), Vec::<i64>::new());
    }

    // Test that binding with a function that filters some elements works correctly
    #[test]
    fn prop_filtering(xs in prop::collection::vec(-100..100i64, 0..10)) {
      let m = xs.clone();

      // Using filter_positive, which only keeps positive values
      let expected: Vec<i64> = xs.into_iter().filter(|x| *x > 0).collect();
      let result = m.bind(filter_positive);

      prop_assert_eq!(result, expected);
    }
  }
}
