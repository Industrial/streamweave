//! Implementation of the `Monad` trait for `Option<A>`.
//!
//! This module implements the monadic bind operation for the `Option` type,
//! allowing `Option` values to be chained in a way that automatically handles
//! the `None` case, similar to Haskell's `Maybe` monad.
//!
//! The implementation uses pattern matching to:
//! - Apply the provided function to the value inside `Some`, returning the result
//! - Short-circuit when encountering `None`, immediately returning `None`
//!
//! This enables concise error handling and composition of operations that might fail.

use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe> Monad<A> for Option<A> {
  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    match self {
      Some(a) => f(&a),
      None => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::applicative::Applicative;
  use proptest::prelude::*;

  // Test functions for the monad laws
  fn add_one(x: &i64) -> Option<i64> {
    Some(x + 1)
  }

  fn double(x: &i64) -> Option<i64> {
    Some(x * 2)
  }

  fn safe_div(x: &i64) -> Option<i64> {
    if *x != 0 {
      Some(100 / x)
    } else {
      None
    }
  }

  fn safe_sqrt(x: &i64) -> Option<i64> {
    if *x >= 0 {
      Some(((*x as f64).sqrt()) as i64)
    } else {
      None
    }
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i64;
    let f = add_one;

    let left = Option::<i64>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(left, right);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let m = Some(42i64);
    
    let left = m.clone().bind(|x| Option::<i64>::pure(*x));
    let right = m;

    assert_eq!(left, right);

    // Also test with None
    let none: Option<i64> = None;
    let left = none.clone().bind(|x| Option::<i64>::pure(*x));
    
    assert_eq!(left, none);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m = Some(42i64);
    let f = add_one;
    let g = double;

    let left = m.clone().bind(f).bind(g);
    
    // Create a new closure that doesn't capture f and g directly
    let right = m.bind(|x| {
      let f_result = add_one(x);
      f_result.bind(double)
    });

    assert_eq!(left, right);

    // Also test with None
    let none: Option<i64> = None;
    let left = none.clone().bind(f).bind(g);
    
    // Create a new closure that doesn't capture f and g directly
    let right = none.bind(|x| {
      let f_result = add_one(x);
      f_result.bind(double)
    });
    
    assert_eq!(left, right);
  }

  // Test with None values
  #[test]
  fn test_with_none() {
    let none: Option<i64> = None;
    let f = add_one;

    assert_eq!(none.bind(f), None);
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let m = Some(10i64);

    let result = m.bind(add_one).bind(double).bind(safe_div);
    assert_eq!(result, Some(100 / 22)); // (10+1)*2 = 22, 100/22 = 4

    // Test short-circuiting with None
    let result = m.bind(|_| None::<i64>).bind(double);
    assert_eq!(result, None);
  }

  // Test with functions that can return None
  #[test]
  fn test_with_failing_functions() {
    // Division by zero should produce None
    let m = Some(0i64);
    let result = m.bind(safe_div);
    assert_eq!(result, None);
    
    // Negative square root should produce None
    let m = Some(-4i64);
    let result = m.bind(safe_sqrt);
    assert_eq!(result, None);
    
    // Chaining operations that might fail
    let m = Some(16i64);
    let result = m.bind(safe_sqrt).bind(safe_div); // sqrt(16) = 4, 100/4 = 25
    assert_eq!(result, Some(25));
  }

  // Test combining bind with then
  #[test]
  fn test_bind_then() {
    let m = Some(10i64);
    let n = Some(20i64);

    let result = m.bind(add_one).then(n.bind(double));
    assert_eq!(result, Some(40)); // Discards 10+1=11, returns 20*2=40
    
    // If left side is None, then should produce None
    let none: Option<i64> = None;
    let result = none.then(n.bind(double));
    assert_eq!(result, None);
    
    // If right side is None, then should produce None
    let result = m.then(None::<i64>);
    assert_eq!(result, None);
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m = Some(10i64);
    
    // flat_map should behave the same as bind
    let bind_result = m.clone().bind(add_one);
    let flat_map_result = m.flat_map(add_one);
    
    assert_eq!(bind_result, flat_map_result);
  }

  // Property-based tests
  proptest! {
    // Left identity law
    #[test]
    fn prop_left_identity(a in -1000..1000i64) {
      let left = Option::<i64>::pure(a).bind(add_one);
      let right = add_one(&a);
      
      prop_assert_eq!(left, right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(a in -1000..1000i64, is_some in proptest::bool::ANY) {
      let m = if is_some { Some(a) } else { None };
      
      let left = m.clone().bind(|x| Option::<i64>::pure(*x));
      let right = m;
      
      prop_assert_eq!(left, right);
    }

    // Associativity law
    #[test]
    fn prop_associativity(a in -1000..1000i64, is_some in proptest::bool::ANY, 
                           f_choice in 0..4u8, g_choice in 0..4u8) {
      let m = if is_some { Some(a) } else { None };
      
      // Choose different functions based on f_choice and g_choice
      let f = match f_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => safe_sqrt,
      };
      
      let g = match g_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => safe_sqrt,
      };
      
      let left = m.clone().bind(f).bind(g);
      
      // Use a new closure to avoid referencing f and g variables
      let f_copy = f;
      let g_copy = g;
      let right = m.bind(move |x| {
        let f_result = f_copy(x);
        f_result.bind(g_copy)
      });
      
      // They should be equal (though we might hit None due to safe_div or safe_sqrt)
      prop_assert_eq!(left, right);
    }
    
    // Test that bind preserves None values
    #[test]
    fn prop_none_preserved(f_choice in 0..4u8) {
      let none: Option<i64> = None;
      
      let f = match f_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => safe_sqrt,
      };
      
      let result = none.bind(f);
      prop_assert_eq!(result, None);
    }
    
    // Test that then preserves monadic effects
    #[test]
    fn prop_then_effects(a in -1000..1000i64, b in -1000..1000i64, 
                           is_a_some in proptest::bool::ANY, 
                           is_b_some in proptest::bool::ANY) {
      let m_a = if is_a_some { Some(a) } else { None };
      let m_b = if is_b_some { Some(b) } else { None };
      
      let result = m_a.then(m_b);
      
      // then should produce None if either side is None
      if !is_a_some || !is_b_some {
        prop_assert_eq!(result, None);
      } else {
        prop_assert_eq!(result, Some(b));
      }
    }
    
    // Test that flat_map is equivalent to bind
    #[test]
    fn prop_flat_map_equiv_bind(a in -1000..1000i64, is_some in proptest::bool::ANY,
                                 f_choice in 0..4u8) {
      let m = if is_some { Some(a) } else { None };
      
      let f = match f_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => safe_sqrt,
      };
      
      let bind_result = m.clone().bind(f);
      let flat_map_result = m.flat_map(f);
      
      prop_assert_eq!(bind_result, flat_map_result);
    }
  }
}
