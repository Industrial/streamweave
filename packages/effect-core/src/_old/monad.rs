//! Monad trait and implementations.
//!
//! A monad is an applicative functor that supports flat mapping.

use crate::applicative::Applicative;
use std::sync::{Mutex, RwLock};

/// A monad is an applicative functor that can sequence computations.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The higher-kinded type MonadSelf<U> must implement Send + Sync + 'static
/// - The function type F must implement Send + Sync + 'static
/// - The bound type U must implement Send + Sync + 'static
pub trait Monad<T: Send + Sync + 'static>: Applicative<T> {
  type MonadSelf<U: Send + Sync + 'static>: Send + Sync + 'static;

  fn bind<U, F>(self, f: F) -> Self::MonadSelf<U>
  where
    F: FnMut(T) -> Self::MonadSelf<U> + Send + Sync + 'static,
    U: Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Monad<T> for Option<T> {
  type MonadSelf<U: Send + Sync + 'static> = Option<U>;

  fn bind<U, F>(self, mut f: F) -> Self::MonadSelf<U>
  where
    F: FnMut(T) -> Self::MonadSelf<U> + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    self.and_then(f)
  }
}

// Implementation for Result
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Monad<T> for Result<T, E> {
  type MonadSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn bind<U, F>(self, mut f: F) -> Self::MonadSelf<U>
  where
    F: FnMut(T) -> Self::MonadSelf<U> + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    self.and_then(f)
  }
}

// Implementation for Mutex
impl<T: Send + Sync + 'static> Monad<T> for Mutex<T> {
  type MonadSelf<U: Send + Sync + 'static> = Mutex<U>;

  fn bind<U, F>(self, mut f: F) -> Self::MonadSelf<U>
  where
    F: FnMut(T) -> Self::MonadSelf<U> + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    let inner = self.into_inner().unwrap();
    Mutex::new(f(inner).into_inner().unwrap())
  }
}

// Implementation for RwLock
impl<T: Send + Sync + 'static> Monad<T> for RwLock<T> {
  type MonadSelf<U: Send + Sync + 'static> = RwLock<U>;

  fn bind<U, F>(self, mut f: F) -> Self::MonadSelf<U>
  where
    F: FnMut(T) -> Self::MonadSelf<U> + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    let inner = self.into_inner().unwrap();
    RwLock::new(f(inner).into_inner().unwrap())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::thread;

  // Define test functions that implement Debug
  const FUNCTIONS: &[fn(i32) -> Option<i32>] = &[
    |x| Some(x + 1),
    |x| Some(x * 2),
    |x| Some(x - 1),
    |x| Some(x / 2),
    |x| Some(x * x),
    |x| Some(-x),
  ];

  // Test helper for thread safety
  fn test_thread_safety<F, T>(f: F, input: T) -> T
  where
    F: Fn(T) -> T + Send + Sync + 'static,
    T: Send + Sync + 'static,
  {
    let handle = thread::spawn(move || f(input));
    handle.join().unwrap()
  }

  proptest! {
      #[test]
      fn test_option_monad_identity(x in any::<i32>()) {
          let id = |x: i32| Some(x);
          let result = Some(x).bind(id);
          assert_eq!(result, Some(x));
      }

      #[test]
      fn test_option_monad_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Some(x).bind(f).bind(g);
          let result2 = Some(x).bind(|x| f(x).bind(g));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_result_monad_identity(x in any::<i32>()) {
          let id = |x: i32| Ok::<i32, &str>(x);
          let result: Result<i32, &str> = Ok(x).bind(id);
          assert_eq!(result, Ok(x));
      }

      #[test]
      fn test_result_monad_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = |x: i32| Ok::<i32, &str>(FUNCTIONS[f_idx](x).unwrap());
          let g = |x: i32| Ok::<i32, &str>(FUNCTIONS[g_idx](x).unwrap());
          let result1: Result<i32, &str> = Ok(x).bind(f).bind(g);
          let result2 = Ok(x).bind(|x| f(x).bind(g));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_mutex_monad_identity(x in any::<i32>()) {
          let id = |x: i32| Mutex::new(x);
          let result = Mutex::new(x).bind(id);
          assert_eq!(*result.lock().unwrap(), x);
      }

      #[test]
      fn test_mutex_monad_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = |x: i32| Mutex::new(FUNCTIONS[f_idx](x).unwrap());
          let g = |x: i32| Mutex::new(FUNCTIONS[g_idx](x).unwrap());
          let result1 = Mutex::new(x).bind(f).bind(g);
          let result2 = Mutex::new(x).bind(|x| f(x).bind(g));
          assert_eq!(*result1.lock().unwrap(), *result2.lock().unwrap());
      }

      #[test]
      fn test_rwlock_monad_identity(x in any::<i32>()) {
          let id = |x: i32| RwLock::new(x);
          let result = RwLock::new(x).bind(id);
          assert_eq!(*result.read().unwrap(), x);
      }

      #[test]
      fn test_rwlock_monad_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = |x: i32| RwLock::new(FUNCTIONS[f_idx](x).unwrap());
          let g = |x: i32| RwLock::new(FUNCTIONS[g_idx](x).unwrap());
          let result1 = RwLock::new(x).bind(f).bind(g);
          let result2 = RwLock::new(x).bind(|x| f(x).bind(g));
          assert_eq!(*result1.read().unwrap(), *result2.read().unwrap());
      }

      #[test]
      fn test_thread_safety_properties(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = |x: i32| Some(FUNCTIONS[f_idx](x).unwrap());
          let result = test_thread_safety(|x| Some(x).bind(f), x);
          assert_eq!(result, Some(FUNCTIONS[f_idx](x).unwrap()));

          let r: Result<i32, &str> = Ok(x);
          let f = |x: i32| Ok::<i32, &str>(FUNCTIONS[f_idx](x).unwrap());
          let result = test_thread_safety(|r| r.bind(f), r);
          assert_eq!(result, Ok(FUNCTIONS[f_idx](x).unwrap()));
      }
  }
}
