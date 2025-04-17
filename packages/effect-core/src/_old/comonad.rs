//! Comonad trait and implementations.
//!
//! A comonad is a functor that supports extracting values and extending computations.

use crate::functor::Functor;
use std::sync::{Mutex, RwLock};

/// The Comonad trait represents a type that can extract values and extend computations.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement Send + Sync + 'static
/// - The type parameter B must implement Send + Sync + 'static
/// - The function F must implement Send + Sync + 'static
pub trait Comonad<A: Send + Sync + 'static>: Functor<A> {
  /// Extracts a value from the comonad
  fn extract(self) -> A;

  /// Extends a computation over the comonad
  fn extend<B: Send + Sync + 'static, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static;
}

// Implementation for Option
impl<A: Send + Sync + 'static> Comonad<A> for Option<A> {
  fn extract(self) -> A {
    self.unwrap()
  }

  fn extend<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
  {
    match self {
      Some(a) => Some(f(a)),
      None => None,
    }
  }
}

// Implementation for Result
impl<A: Send + Sync + 'static, E: Send + Sync + 'static> Comonad<A> for Result<A, E> {
  fn extract(self) -> A {
    self.unwrap()
  }

  fn extend<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
  {
    match self {
      Ok(a) => Ok(f(a)),
      Err(e) => Err(e),
    }
  }
}

// Implementation for Mutex
impl<A: Send + Sync + 'static> Comonad<A> for Mutex<A> {
  fn extract(self) -> A {
    self.into_inner().unwrap()
  }

  fn extend<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
  {
    let a = self.into_inner().unwrap();
    Mutex::new(f(a))
  }
}

// Implementation for RwLock
impl<A: Send + Sync + 'static> Comonad<A> for RwLock<A> {
  fn extract(self) -> A {
    self.into_inner().unwrap()
  }

  fn extend<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
  {
    let a = self.into_inner().unwrap();
    RwLock::new(f(a))
  }
}

// Implementation for Arc
impl<A: Send + Sync + 'static> Comonad<A> for std::sync::Arc<A> {
  fn extract(self) -> A {
    match std::sync::Arc::try_unwrap(self) {
      Ok(a) => a,
      Err(_) => panic!("Cannot extract value from Arc with multiple references"),
    }
  }

  fn extend<B: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B + Send + Sync + 'static,
  {
    let a = self.extract();
    std::sync::Arc::new(f(a))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Fixed set of functions for testing
  const FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x,     // identity
    |x| x + 1, // increment
    |x| x * 2, // double
  ];

  proptest! {
      #[test]
      fn test_option_extract(x in any::<i32>()) {
          let m = Some(x);
          assert_eq!(m.extract(), x);
      }

      #[test]
      fn test_result_extract(x in any::<i32>()) {
          let m: Result<i32, &str> = Ok(x);
          assert_eq!(m.extract(), x);
      }

      #[test]
      fn test_mutex_extract(x in any::<i32>()) {
          let m = Mutex::new(x);
          assert_eq!(m.extract(), x);
      }

      #[test]
      fn test_rwlock_extract(x in any::<i32>()) {
          let m = RwLock::new(x);
          assert_eq!(m.extract(), x);
      }

      #[test]
      fn test_arc_extract(x in any::<i32>()) {
          let m = std::sync::Arc::new(x);
          assert_eq!(m.extract(), x);
      }

      #[test]
      fn test_option_extend(
          f_idx in 0..3usize,
          x in any::<i32>()
      ) {
          let f = |a: i32| FUNCTIONS[f_idx](a);
          let m = Some(x);
          let result = m.extend(f);
          assert_eq!(result, Some(FUNCTIONS[f_idx](x)));
      }

      #[test]
      fn test_result_extend(
          f_idx in 0..3usize,
          x in any::<i32>()
      ) {
          let f = |a: i32| FUNCTIONS[f_idx](a);
          let m: Result<i32, &str> = Ok(x);
          let result = m.extend(f);
          assert_eq!(result, Ok(FUNCTIONS[f_idx](x)));
      }

      #[test]
      fn test_mutex_extend(
          f_idx in 0..3usize,
          x in any::<i32>()
      ) {
          let f = |a: i32| FUNCTIONS[f_idx](a);
          let m = Mutex::new(x);
          let result = m.extend(f);
          assert_eq!(result.into_inner().unwrap(), FUNCTIONS[f_idx](x));
      }

      #[test]
      fn test_rwlock_extend(
          f_idx in 0..3usize,
          x in any::<i32>()
      ) {
          let f = |a: i32| FUNCTIONS[f_idx](a);
          let m = RwLock::new(x);
          let result = m.extend(f);
          assert_eq!(result.into_inner().unwrap(), FUNCTIONS[f_idx](x));
      }

      #[test]
      fn test_arc_extend(
          f_idx in 0..3usize,
          x in any::<i32>()
      ) {
          let f = |a: i32| FUNCTIONS[f_idx](a);
          let m = std::sync::Arc::new(x);
          let result = m.extend(f);
          assert_eq!(result.extract(), FUNCTIONS[f_idx](x));
      }
  }
}
