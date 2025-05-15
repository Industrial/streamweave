//! Implementation of the `Monad` trait for `Result<T, E>`.
//!
//! This module implements the monadic bind operation for the `Result` type,
//! allowing `Result` values to be chained in a way that automatically handles
//! error propagation. This enables clean error handling and composition of fallible operations.
//!
//! The implementation uses pattern matching to:
//! - Apply the provided function to the value inside `Ok`, returning the result
//! - Short-circuit when encountering `Err`, immediately returning the error
//!
//! This enables concise error handling without verbose error propagation code.

use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Monad<T> for Result<T, E> {
  fn bind<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> Self::HigherSelf<U> + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    match self {
      Ok(t) => f(&t),
      Err(e) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::applicative::Applicative;
  use proptest::prelude::*;

  // Test functions for the monad laws
  fn add_one(x: &i64) -> Result<i64, String> {
    Ok(x + 1)
  }

  fn double(x: &i64) -> Result<i64, String> {
    Ok(x * 2)
  }

  fn safe_div(x: &i64) -> Result<i64, String> {
    if *x != 0 {
      Ok(100 / x)
    } else {
      Err("Division by zero".to_string())
    }
  }

  fn validate_positive(x: &i64) -> Result<i64, String> {
    if *x > 0 {
      Ok(*x)
    } else {
      Err("Value must be positive".to_string())
    }
  }

  // Left identity law: pure(a).bind(f) == f(a)
  #[test]
  fn test_left_identity() {
    let a = 42i64;
    let f = add_one;

    let left = Result::<i64, String>::pure(a).bind(f);
    let right = f(&a);

    assert_eq!(left, right);
  }

  // Right identity law: m.bind(pure) == m
  #[test]
  fn test_right_identity() {
    let m: Result<i64, String> = Ok(42);
    
    let left = m.clone().bind(|x| Result::<i64, String>::pure(*x));
    let right = m;

    assert_eq!(left, right);

    // Also test with Err
    let e: Result<i64, String> = Err("some error".to_string());
    let left = e.clone().bind(|x| Result::<i64, String>::pure(*x));
    
    assert_eq!(left, e);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let m: Result<i64, String> = Ok(42);
    let f = add_one;
    let g = double;

    let left = m.clone().bind(f).bind(g);
    
    // Create a new closure that doesn't capture f and g directly
    let right = m.bind(|x| {
      let f_result = add_one(x);
      f_result.bind(double)
    });

    assert_eq!(left, right);

    // Also test with Err
    let e: Result<i64, String> = Err("some error".to_string());
    let left = e.clone().bind(f).bind(g);
    
    // Create a new closure that doesn't capture f and g directly
    let right = e.bind(|x| {
      let f_result = add_one(x);
      f_result.bind(double)
    });
    
    assert_eq!(left, right);
  }

  // Test with Err values
  #[test]
  fn test_with_err() {
    let e: Result<i64, String> = Err("some error".to_string());
    let f = add_one;

    assert_eq!(e.bind(f), Err("some error".to_string()));
  }

  // Test chaining operations with bind
  #[test]
  fn test_bind_chaining() {
    let m: Result<i64, String> = Ok(10);

    let result = m.clone().bind(add_one).bind(double).bind(safe_div);
    assert_eq!(result, Ok(100 / 22)); // (10+1)*2 = 22, 100/22 = 4

    // Test short-circuiting with Err
    let result = m.bind(|_| Err::<i64, String>("short-circuit".to_string())).bind(double);
    assert_eq!(result, Err("short-circuit".to_string()));
  }

  // Test with functions that can return Err
  #[test]
  fn test_with_failing_functions() {
    // Division by zero should produce Err
    let m: Result<i64, String> = Ok(0);
    let result = m.bind(safe_div);
    assert_eq!(result, Err("Division by zero".to_string()));
    
    // Negative value should produce Err
    let m: Result<i64, String> = Ok(-4);
    let result = m.bind(validate_positive);
    assert_eq!(result, Err("Value must be positive".to_string()));
    
    // Chaining operations that might fail
    let m: Result<i64, String> = Ok(5);
    let result = m.bind(validate_positive).bind(safe_div); // 5 is positive, 100/5 = 20
    assert_eq!(result, Ok(20));
  }

  // Test error propagation and priority
  #[test]
  fn test_error_propagation() {
    // The first error should be propagated
    let m: Result<i64, String> = Ok(0);
    let result = m.bind(safe_div).bind(validate_positive);
    assert_eq!(result, Err("Division by zero".to_string()));
    
    // Error type should be preserved
    let m: Result<i64, &str> = Err("original error");
    let result = m.bind(|x| Ok(x + 1));
    assert_eq!(result, Err("original error"));
  }

  // Test combining bind with then
  #[test]
  fn test_bind_then() {
    let m: Result<i64, String> = Ok(10);
    let n: Result<i64, String> = Ok(20);

    let result = m.clone().bind(add_one).then(n.clone().bind(double));
    assert_eq!(result, Ok(40)); // Discards 10+1=11, returns 20*2=40
    
    // If left side is Err, then should produce Err
    let e: Result<i64, String> = Err("left error".to_string());
    let result = e.then(n.clone().bind(double));
    assert_eq!(result, Err("left error".to_string()));
    
    // If right side is Err, then should produce Err
    let result = m.then(Err::<i64, String>("right error".to_string()));
    assert_eq!(result, Err("right error".to_string()));
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let m: Result<i64, String> = Ok(10);
    
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
      let left = Result::<i64, String>::pure(a).bind(add_one);
      let right = add_one(&a);
      
      prop_assert_eq!(left, right);
    }

    // Right identity law
    #[test]
    fn prop_right_identity(a in -1000..1000i64, is_ok in proptest::bool::ANY) {
      let m: Result<i64, String> = if is_ok { Ok(a) } else { Err("error".to_string()) };
      
      let left = m.clone().bind(|x| Result::<i64, String>::pure(*x));
      let right = m;
      
      prop_assert_eq!(left, right);
    }

    // Associativity law
    #[test]
    fn prop_associativity(a in -1000..1000i64, is_ok in proptest::bool::ANY, 
                           f_choice in 0..4u8, g_choice in 0..4u8) {
      let m: Result<i64, String> = if is_ok { Ok(a) } else { Err("error".to_string()) };
      
      // Choose different functions based on f_choice and g_choice
      let f = match f_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => validate_positive,
      };
      
      let g = match g_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => validate_positive,
      };
      
      let left = m.clone().bind(f).bind(g);
      
      // Use a new closure to avoid referencing f and g variables
      let f_copy = f;
      let g_copy = g;
      let right = m.bind(move |x| {
        let f_result = f_copy(x);
        f_result.bind(g_copy)
      });
      
      // They should be equal
      prop_assert_eq!(left, right);
    }
    
    // Test that bind preserves Err values
    #[test]
    fn prop_err_preserved(error_message in "\\PC*", f_choice in 0..4u8) {
      let err: Result<i64, String> = Err(error_message.clone());
      
      let f = match f_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => validate_positive,
      };
      
      let result = err.bind(f);
      prop_assert_eq!(result, Err(error_message));
    }
    
    // Test that then preserves monadic effects
    #[test]
    fn prop_then_effects(a in -1000..1000i64, b in -1000..1000i64, 
                          is_a_ok in proptest::bool::ANY, 
                          is_b_ok in proptest::bool::ANY) {
      let error_a = "error_a".to_string();
      let error_b = "error_b".to_string();
      
      let m_a: Result<i64, String> = if is_a_ok { Ok(a) } else { Err(error_a.clone()) };
      let m_b: Result<i64, String> = if is_b_ok { Ok(b) } else { Err(error_b.clone()) };
      
      let result = m_a.clone().then(m_b);
      
      // then should produce Err if either side is Err, with the first error taking precedence
      if !is_a_ok {
        prop_assert_eq!(result, Err(error_a));
      } else if !is_b_ok {
        prop_assert_eq!(result, Err(error_b));
      } else {
        prop_assert_eq!(result, Ok(b));
      }
    }
    
    // Test that flat_map is equivalent to bind
    #[test]
    fn prop_flat_map_equiv_bind(a in -1000..1000i64, is_ok in proptest::bool::ANY,
                                 f_choice in 0..4u8) {
      let m: Result<i64, String> = if is_ok { Ok(a) } else { Err("error".to_string()) };
      
      let f = match f_choice {
        0 => add_one,
        1 => double,
        2 => safe_div,
        _ => validate_positive,
      };
      
      let bind_result = m.clone().bind(f);
      let flat_map_result = m.flat_map(f);
      
      prop_assert_eq!(bind_result, flat_map_result);
    }
  }
} 