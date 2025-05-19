//! Implementation of the `Monad` trait for `LinkedList<A>`.
//!
//! This module implements the monadic bind operation for the `LinkedList` type,
//! enabling non-deterministic computation where each element in the list
//! can produce multiple results.
//!
//! The implementation flattens a nested list, representing the application
//! of a function that returns multiple possibilities to a collection of
//! values. This is similar to the Vec and VecDeque monads, but with the performance
//! characteristics of a doubly-linked list.

use crate::impls::linkedlist::category::LinkedListCategory;
use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

// Implement Monad for LinkedListCategory, not for LinkedList directly
impl<A: CloneableThreadSafe> Monad<A> for LinkedListCategory {
  fn bind<B, F>(self, _f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is a placeholder implementation since binding
    // is done through the extension trait
    LinkedListCategory
  }
}

// Extension trait to make LinkedList monadic operations more ergonomic
pub trait LinkedListMonadExt<A: CloneableThreadSafe> {
  fn bind<B, F>(self, f: F) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a A) -> LinkedList<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;

  fn flat_map<B, F>(self, f: F) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a A) -> LinkedList<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

// Implement the extension trait for LinkedList
impl<A: CloneableThreadSafe> LinkedListMonadExt<A> for LinkedList<A> {
  fn bind<B, F>(self, mut f: F) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a A) -> LinkedList<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = LinkedList::new();
    for a in self {
      result.append(&mut f(&a));
    }
    result
  }

  fn flat_map<B, F>(self, f: F) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a A) -> LinkedList<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    self.bind(f)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::impls::linkedlist::applicative::LinkedListApplicativeExt;
  use proptest::prelude::*;

  // Helper function to convert a Vec to a LinkedList
  fn to_linkedlist<T: Clone>(vec: Vec<T>) -> LinkedList<T> {
    vec.into_iter().collect()
  }

  // Test functions for the monad laws
  fn add_one_to_many(x: &i64) -> LinkedList<i64> {
    to_linkedlist(vec![x + 1, x + 2])
  }

  fn multiply_by(x: &i64) -> LinkedList<i64> {
    to_linkedlist(vec![x * 2, x * 3])
  }

  // Function that returns an empty LinkedList for some inputs
  fn filter_positive(x: &i64) -> LinkedList<i64> {
    if *x > 0 {
      let mut result = LinkedList::new();
      result.push_back(*x);
      result
    } else {
      LinkedList::new()
    }
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i64;
    let f = add_one_to_many;

    let left = LinkedList::<i64>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(left, right);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let m = to_linkedlist(vec![1, 2, 3]);

    let left = m.clone().bind(|x| LinkedList::<i64>::pure(*x));
    let right = m;

    assert_eq!(left, right);

    // Also test with empty list
    let empty: LinkedList<i64> = LinkedList::new();
    let left = empty.clone().bind(|x| LinkedList::<i64>::pure(*x));

    assert_eq!(left, empty);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m = to_linkedlist(vec![5, 10]);
    let f = add_one_to_many;
    let g = multiply_by;

    let left = m.clone().bind(f).bind(g);

    let right = m.bind(|x| {
      let f_result = add_one_to_many(x);
      f_result.bind(multiply_by)
    });

    assert_eq!(left, right);

    // Also test with empty list
    let empty: LinkedList<i64> = LinkedList::new();
    let left = empty.clone().bind(f).bind(g);

    let right = empty.bind(|x| {
      let f_result = add_one_to_many(x);
      f_result.bind(multiply_by)
    });

    assert_eq!(left, right);
  }

  // Test with empty list
  #[test]
  fn test_with_empty() {
    let empty: LinkedList<i64> = LinkedList::new();
    let f = add_one_to_many;

    assert_eq!(empty.bind(f), LinkedList::new());
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let m = to_linkedlist(vec![1, 2]);

    // 1 -> [2, 3], 2 -> [3, 4], then each multiplied by 2 and 3
    let result = m.bind(add_one_to_many).bind(multiply_by);

    // Build the expected LinkedList
    let expected = to_linkedlist(vec![
      // From 1 -> 2 -> 4, 6
      4, 6, // From 1 -> 3 -> 6, 9
      6, 9, // From 2 -> 3 -> 6, 9
      6, 9, // From 2 -> 4 -> 8, 12
      8, 12,
    ]);

    assert_eq!(result, expected);
  }

  // Test with functions that can return empty lists (filtering)
  #[test]
  fn test_with_filtering_functions() {
    let m = to_linkedlist(vec![-2, 0, 3, 5]);

    // Should filter out negative and zero values
    let result = m.clone().bind(filter_positive);

    let expected = to_linkedlist(vec![3, 5]);
    assert_eq!(result, expected);

    // Chain with another operation
    let result = m.bind(filter_positive).bind(multiply_by);

    let expected = to_linkedlist(vec![6, 9, 10, 15]);
    assert_eq!(result, expected);
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m = to_linkedlist(vec![1, 2]);

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
      let left = LinkedList::<i64>::pure(a).bind(add_one_to_many);
      let right = add_one_to_many(&a);

      prop_assert_eq!(left, right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let list = to_linkedlist(xs);

      let left = list.clone().bind(|x| LinkedList::<i64>::pure(*x));
      let right = list;

      prop_assert_eq!(left, right);
    }

    // Associativity law
    #[test]
    fn prop_associativity(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let list = to_linkedlist(xs);

      let f = add_one_to_many;
      let g = multiply_by;

      let left = list.clone().bind(f).bind(g);

      let right = list.bind(|x| {
        let f_result = add_one_to_many(x);
        f_result.bind(multiply_by)
      });

      prop_assert_eq!(left, right);
    }

    // Test that bind with a function returning an empty LinkedList for all inputs produces an empty LinkedList
    #[test]
    fn prop_empty_result(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let list = to_linkedlist(xs);

      let empty_fn = |_: &i64| LinkedList::<i64>::new();

      prop_assert_eq!(list.bind(empty_fn), LinkedList::<i64>::new());
    }

    // Test that filtering works consistently
    #[test]
    fn prop_filtering(xs in prop::collection::vec(-100..100i64, 0..10)) {
      let list = to_linkedlist(xs.clone());

      let result = list.bind(filter_positive);

      // Manually filter the original vector and create a LinkedList
      let expected = to_linkedlist(xs.into_iter().filter(|&x| x > 0).collect::<Vec<_>>());

      prop_assert_eq!(result, expected);
    }
  }
}
