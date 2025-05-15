//! Implementation of the `Monad` trait for `VecDeque<A>`.
//!
//! This module implements the monadic bind operation for the `VecDeque` type,
//! enabling non-deterministic computation where each element in the deque
//! can produce multiple results.
//!
//! The implementation flattens a nested deque, representing the application
//! of a function that returns multiple possibilities to a collection of
//! values. This is similar to the Vec monad, but with the performance
//! characteristics of a double-ended queue.

use crate::impls::vecdeque::category::VecDequeCategory;
use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::VecDeque;

// Implement Monad for VecDequeCategory, not for VecDeque directly
impl<A: CloneableThreadSafe> Monad<A> for VecDequeCategory {
  fn bind<B, F>(self, _f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is a placeholder implementation since binding
    // is done through the extension trait
    VecDeque::new()
  }
}

// Extension trait to make VecDeque monadic operations more ergonomic
pub trait VecDequeMonadExt<A: CloneableThreadSafe> {
  fn bind<B, F>(self, f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a A) -> VecDeque<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
    
  fn flat_map<B, F>(self, f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a A) -> VecDeque<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

// Implement the extension trait for VecDeque
impl<A: CloneableThreadSafe> VecDequeMonadExt<A> for VecDeque<A> {
  fn bind<B, F>(self, mut f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a A) -> VecDeque<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = VecDeque::new();
    for a in self {
      result.extend(f(&a));
    }
    result
  }
  
  fn flat_map<B, F>(self, f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a A) -> VecDeque<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    self.bind(f)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::impls::vecdeque::applicative::VecDequeApplicativeExt;
  use proptest::prelude::*;

  // Test functions for the monad laws
  fn add_one_to_many(x: &i64) -> VecDeque<i64> {
    let mut result = VecDeque::new();
    result.push_back(x + 1);
    result.push_back(x + 2);
    result
  }

  fn multiply_by(x: &i64) -> VecDeque<i64> {
    let mut result = VecDeque::new();
    result.push_back(x * 2);
    result.push_back(x * 3);
    result
  }

  // Function that returns an empty VecDeque for some inputs
  fn filter_positive(x: &i64) -> VecDeque<i64> {
    if *x > 0 {
      let mut result = VecDeque::new();
      result.push_back(*x);
      result
    } else {
      VecDeque::new()
    }
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i64;
    let f = add_one_to_many;

    let left = VecDeque::<i64>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(left, right);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let mut m = VecDeque::new();
    m.push_back(1);
    m.push_back(2);
    m.push_back(3);
    
    let left = m.clone().bind(|x| VecDeque::<i64>::pure(*x));
    let right = m;

    assert_eq!(left, right);

    // Also test with empty deque
    let empty: VecDeque<i64> = VecDeque::new();
    let left = empty.clone().bind(|x| VecDeque::<i64>::pure(*x));
    
    assert_eq!(left, empty);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let mut m = VecDeque::new();
    m.push_back(5);
    m.push_back(10);
    
    let f = add_one_to_many;
    let g = multiply_by;

    let left = m.clone().bind(f).bind(g);
    
    let right = m.bind(|x| {
      let f_result = add_one_to_many(x);
      f_result.bind(multiply_by)
    });

    assert_eq!(left, right);

    // Also test with empty deque
    let empty: VecDeque<i64> = VecDeque::new();
    let left = empty.clone().bind(f).bind(g);
    
    let right = empty.bind(|x| {
      let f_result = add_one_to_many(x);
      f_result.bind(multiply_by)
    });
    
    assert_eq!(left, right);
  }

  // Test with empty deque
  #[test]
  fn test_with_empty() {
    let empty: VecDeque<i64> = VecDeque::new();
    let f = add_one_to_many;

    assert_eq!(empty.bind(f), VecDeque::new());
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let mut m = VecDeque::new();
    m.push_back(1);
    m.push_back(2);

    // 1 -> [2, 3], 2 -> [3, 4], then each multiplied by 2 and 3
    let result = m.bind(add_one_to_many).bind(multiply_by);
    
    let mut expected = VecDeque::new();
    // From 1 -> 2 -> 4, 6
    expected.push_back(4);
    expected.push_back(6);
    // From 1 -> 3 -> 6, 9
    expected.push_back(6);
    expected.push_back(9);
    // From 2 -> 3 -> 6, 9
    expected.push_back(6);
    expected.push_back(9);
    // From 2 -> 4 -> 8, 12
    expected.push_back(8);
    expected.push_back(12);
    
    assert_eq!(result, expected);
  }

  // Test with functions that can return empty deques (filtering)
  #[test]
  fn test_with_filtering_functions() {
    let mut m = VecDeque::new();
    m.push_back(-2);
    m.push_back(0);
    m.push_back(3);
    m.push_back(5);
    
    // Should filter out negative and zero values
    let result = m.clone().bind(filter_positive);
    
    let mut expected = VecDeque::new();
    expected.push_back(3);
    expected.push_back(5);
    
    assert_eq!(result, expected);
    
    // Chain with another operation
    let result = m.bind(filter_positive).bind(multiply_by);
    
    let mut expected = VecDeque::new();
    expected.push_back(6);
    expected.push_back(9);
    expected.push_back(10);
    expected.push_back(15);
    
    assert_eq!(result, expected);
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let mut m = VecDeque::new();
    m.push_back(1);
    m.push_back(2);
    
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
      let left = VecDeque::<i64>::pure(a).bind(add_one_to_many);
      let right = add_one_to_many(&a);
      
      prop_assert_eq!(left, right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let mut m = VecDeque::new();
      for x in xs.clone() {
        m.push_back(x);
      }
      
      let left = m.clone().bind(|x| VecDeque::<i64>::pure(*x));
      let right = m;
      
      prop_assert_eq!(left, right);
    }

    // Associativity law
    #[test]
    fn prop_associativity(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let mut m = VecDeque::new();
      for x in xs {
        m.push_back(x);
      }
      
      let f = add_one_to_many;
      let g = multiply_by;
      
      let left = m.clone().bind(f).bind(g);
      
      let right = m.bind(|x| {
        let f_result = add_one_to_many(x);
        f_result.bind(multiply_by)
      });
      
      prop_assert_eq!(left, right);
    }
    
    // Test that bind with a function returning an empty VecDeque for all inputs produces an empty VecDeque
    #[test]
    fn prop_empty_result(xs in prop::collection::vec(-100..100i64, 0..5)) {
      let mut m = VecDeque::new();
      for x in xs {
        m.push_back(x);
      }
      
      let empty_fn = |_: &i64| VecDeque::<i64>::new();
      
      prop_assert_eq!(m.bind(empty_fn), VecDeque::<i64>::new());
    }
    
    // Test that filtering works consistently
    #[test]
    fn prop_filtering(xs in prop::collection::vec(-100..100i64, 0..10)) {
      let mut m = VecDeque::new();
      for x in xs.clone() {
        m.push_back(x);
      }
      
      let result = m.bind(filter_positive);
      
      // Manually filter the original vector and create a VecDeque
      let mut expected = VecDeque::new();
      for x in xs {
        if x > 0 {
          expected.push_back(x);
        }
      }
      
      prop_assert_eq!(result, expected);
    }
  }
} 