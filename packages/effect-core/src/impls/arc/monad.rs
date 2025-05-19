//! Implementation of the `Monad` trait for `Arc<A>`.
//!
//! This module implements the Identity monad for thread-safe reference-counted values,
//! allowing for sequential composition of computations on Arc-wrapped values.
//!
//! The Arc monad provides a thread-safe wrapper around a value with monadic operations:
//! - `bind` applies a function to the contained value and returns a new Arc-wrapped value
//! - All monad laws (left identity, right identity, and associativity) are preserved
//!
//! This implementation demonstrates how Arc can be used in a functional programming context
//! to provide a thread-safe monadic interface for shared ownership values.

use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<A: CloneableThreadSafe> Monad<A> for Arc<A> {
  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    f(&self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::applicative::Applicative;
  use proptest::prelude::*;
  use std::thread;

  // Helper for testing thread safety
  fn test_thread_safety<F, T, U>(f: F, input: T) -> U
  where
    F: FnOnce(T) -> U + Send + 'static,
    T: Send + 'static,
    U: Send + 'static,
  {
    let handle = thread::spawn(move || f(input));
    handle.join().unwrap()
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i32;
    let f = |x: &i32| Arc::new(x + 10);

    let left = Arc::<i32>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(*left, *right);
    assert_eq!(*left, 52);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let m = Arc::new(42i32);
    let pure_fn = |x: &i32| Arc::<i32>::pure(*x);

    let result = m.clone().bind(pure_fn);
    assert_eq!(*result, *m);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m = Arc::new(5i32);
    let f = |x: &i32| Arc::new(x + 1);
    let g = |x: &i32| Arc::new(x * 2);

    let left = m.clone().bind(f).bind(g);

    let right = m.bind(move |x| f(x).bind(g));

    assert_eq!(*left, *right);
    assert_eq!(*left, 12); // (5+1)*2
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let m = Arc::new(10i32);

    // Chain operations
    let result = m
      .bind(|x| Arc::new(x + 1))
      .bind(|x| Arc::new(x * 2))
      .bind(|x| Arc::new(x - 5));

    assert_eq!(*result, 17); // ((10+1)*2)-5 = 17
  }

  // Test with closures that capture their environment
  #[test]
  fn test_closure_capturing() {
    let factor = 3;
    let m = Arc::new(5i32);

    let result = m.bind(move |x| Arc::new(x * factor));

    assert_eq!(*result, 15); // 5*3 = 15
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m = Arc::new(10i32);

    // flat_map should behave the same as bind
    let bind_result = m.clone().bind(|x| Arc::new(x + 5));
    let flat_map_result = m.flat_map(|x| Arc::new(x + 5));

    assert_eq!(*bind_result, *flat_map_result);
  }

  // Test thread safety and concurrent usage
  #[test]
  fn test_thread_safety_bind() {
    let m = Arc::new(7i32);
    let m_clone = m.clone();

    // Run bind in another thread
    let result = test_thread_safety(move |arc| arc.bind(|x| Arc::new(x * 2)), m);

    // Original Arc should remain unchanged
    assert_eq!(*m_clone, 7);

    // Result should have the mapped value
    assert_eq!(*result, 14);
  }

  // Property-based tests
  proptest! {
    // Left identity law
    #[test]
    fn prop_left_identity(a in -100..100i32) {
      let f = |x: &i32| Arc::new(x.saturating_add(10));

      let left = Arc::<i32>::pure(a).bind(f);
      let right = f(&a);

      prop_assert_eq!(*left, *right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(x in -100..100i32) {
      let m = Arc::new(x);
      let pure_fn = |y: &i32| Arc::<i32>::pure(*y);

      let result = m.clone().bind(pure_fn);
      prop_assert_eq!(*result, x);
    }

    // Associativity law
    #[test]
    fn prop_associativity(x in -100..100i32) {
      let m = Arc::new(x);

      let f = |y: &i32| Arc::new(y.saturating_add(1));
      let g = |y: &i32| Arc::new(y.saturating_mul(2));

      let left = m.clone().bind(f).bind(g);
      let right = m.bind(move |y| f(y).bind(g));

      prop_assert_eq!(*left, *right);
    }

    // Test with various operations
    #[test]
    fn prop_bind_operations(x in -100..100i32, y in -10..10i32, z in -5..5i32) {
      // Avoid division by zero
      let z_safe = if z == 0 { 1 } else { z };

      let m = Arc::new(x);

      let result = m.bind(move |a| Arc::new(a.saturating_add(y)))
                    .bind(move |a| Arc::new(a.saturating_mul(z_safe)))
                    .bind(move |a| Arc::new(a.saturating_sub(1)));

      let expected = ((x.saturating_add(y)).saturating_mul(z_safe)).saturating_sub(1);
      prop_assert_eq!(*result, expected);
    }

    // Test flat_map equivalence
    #[test]
    fn prop_flat_map_equiv_bind(x in -100..100i32) {
      let m = Arc::new(x);
      let f = |y: &i32| Arc::new(y.saturating_add(10));

      let bind_result = m.clone().bind(f);
      let flat_map_result = m.flat_map(f);

      prop_assert_eq!(*bind_result, *flat_map_result);
    }

    // Test thread safety with concurrent operations
    #[test]
    fn prop_thread_safety(x in -100..100i32) {
      let m = Arc::new(x);
      let m_clone = m.clone();

      // Run in another thread
      let result = test_thread_safety(
        move |arc| arc.bind(|v| Arc::new(v.saturating_mul(2))),
        m
      );

      // Original Arc should remain unchanged
      prop_assert_eq!(*m_clone, x);

      // Result should have the expected mapped value
      prop_assert_eq!(*result, x.saturating_mul(2));
    }
  }
}
