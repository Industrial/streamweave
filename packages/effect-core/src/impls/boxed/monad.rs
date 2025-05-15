//! Implementation of the `Monad` trait for `Box<A>`.
//!
//! This module implements the Identity monad for boxed values, allowing for
//! sequential composition of computations on boxed values.
//!
//! The Box monad provides a thin wrapper around a value with monadic operations:
//! - `bind` applies a function to the contained value and returns a new boxed value
//! - All monad laws (left identity, right identity, and associativity) are preserved
//!
//! This implementation demonstrates how Box can be used in a functional programming context
//! to provide a monadic interface for boxed values.

use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Monad<A> for Box<A> {
  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    f(&*self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::applicative::Applicative;
  use proptest::prelude::*;

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i32;
    let f = |x: &i32| Box::new(x + 10);

    let left = Box::<i32>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(*left, *right);
    assert_eq!(*left, 52);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let m = Box::new(42i32);
    let pure_fn = |x: &i32| Box::<i32>::pure(*x);
    
    let result = m.clone().bind(pure_fn);
    assert_eq!(*result, *m);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m = Box::new(5i32);
    let f = |x: &i32| Box::new(x + 1);
    let g = |x: &i32| Box::new(x * 2);

    let left = m.clone().bind(f).bind(g);
    
    let right = m.bind(move |x| {
      f(x).bind(g)
    });
    
    assert_eq!(*left, *right);
    assert_eq!(*left, 12); // (5+1)*2
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let m = Box::new(10i32);
    
    // Chain operations
    let result = m.bind(|x| Box::new(x + 1))
                  .bind(|x| Box::new(x * 2))
                  .bind(|x| Box::new(x - 5));
                  
    assert_eq!(*result, 17); // ((10+1)*2)-5 = 17
  }

  // Test with closures that capture their environment
  #[test]
  fn test_closure_capturing() {
    let factor = 3;
    let m = Box::new(5i32);
    
    let result = m.bind(move |x| Box::new(x * factor));
    
    assert_eq!(*result, 15); // 5*3 = 15
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m = Box::new(10i32);
    
    // flat_map should behave the same as bind
    let bind_result = m.clone().bind(|x| Box::new(x + 5));
    let flat_map_result = m.flat_map(|x| Box::new(x + 5));
    
    assert_eq!(*bind_result, *flat_map_result);
  }

  // Property-based tests
  proptest! {
    // Left identity law
    #[test]
    fn prop_left_identity(a in -100..100i32) {
      let f = |x: &i32| Box::new(x.saturating_add(10));
      
      let left = Box::<i32>::pure(a).bind(f);
      let right = f(&a);
      
      prop_assert_eq!(*left, *right);
    }
    
    // Right identity law
    #[test]
    fn prop_right_identity(x in -100..100i32) {
      let m = Box::new(x);
      let pure_fn = |y: &i32| Box::<i32>::pure(*y);
      
      let result = m.clone().bind(pure_fn);
      prop_assert_eq!(*result, x);
    }
    
    // Associativity law
    #[test]
    fn prop_associativity(x in -100..100i32) {
      let m = Box::new(x);
      
      let f = |y: &i32| Box::new(y.saturating_add(1));
      let g = |y: &i32| Box::new(y.saturating_mul(2));
      
      let left = m.clone().bind(f).bind(g);
      let right = m.bind(move |y| f(y).bind(g));
      
      prop_assert_eq!(*left, *right);
    }
    
    // Test with various operations
    #[test]
    fn prop_bind_operations(x in -100..100i32, y in -10..10i32, z in -5..5i32) {
      // Avoid division by zero
      let z_safe = if z == 0 { 1 } else { z };
      
      let m = Box::new(x);
      
      let result = m.bind(move |a| Box::new(a.saturating_add(y)))
                    .bind(move |a| Box::new(a.saturating_mul(z_safe)))
                    .bind(move |a| Box::new(a.saturating_sub(1)));
                    
      let expected = ((x.saturating_add(y)).saturating_mul(z_safe)).saturating_sub(1);
      prop_assert_eq!(*result, expected);
    }
    
    // Test flat_map equivalence
    #[test]
    fn prop_flat_map_equiv_bind(x in -100..100i32) {
      let m = Box::new(x);
      let f = |y: &i32| Box::new(y.saturating_add(10));
      
      let bind_result = m.clone().bind(f);
      let flat_map_result = m.flat_map(f);
      
      prop_assert_eq!(*bind_result, *flat_map_result);
    }
  }
} 