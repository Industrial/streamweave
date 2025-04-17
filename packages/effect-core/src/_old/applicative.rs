//! Applicative trait and implementations.
//!
//! An applicative functor is a functor that can apply functions within the functor context.

use crate::functor::Functor;
use std::sync::{Mutex, RwLock};

/// An applicative functor is a functor that can lift values and apply functions.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The higher-kinded type ApplicativeSelf<U> must implement Send + Sync + 'static
/// - The function type F must implement Send + Sync + 'static
/// - The applied type U must implement Send + Sync + 'static
pub trait Applicative<T: Send + Sync + 'static>: Functor<T> {
  /// The higher-kinded type that results from applying a function
  type ApplicativeSelf<U: Send + Sync + 'static>: Send + Sync + 'static;

  /// Lifts a value into the applicative context
  fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static;

  /// Applies a function within the applicative context
  fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Applicative<T> for Option<T> {
  type ApplicativeSelf<U: Send + Sync + 'static> = Option<U>;

  fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static,
  {
    Some(value)
  }

  fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    match (self, f) {
      (Some(a), Some(mut f)) => Some(f(a)),
      _ => None,
    }
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Applicative<T> for Vec<T> {
  type ApplicativeSelf<U: Send + Sync + 'static> = Vec<U>;

  fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static,
  {
    vec![value]
  }

  fn ap<U, F>(self, fs: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    fs.into_iter()
      .flat_map(|mut func| self.clone().into_iter().map(move |x| func(x)))
      .collect()
  }
}

// Implementation for Result
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Applicative<T> for Result<T, E> {
  type ApplicativeSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static,
  {
    Ok(value)
  }

  fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    match (self, f) {
      (Ok(a), Ok(mut f)) => Ok(f(a)),
      (Err(e), _) => Err(e),
      (_, Err(e)) => Err(e),
    }
  }
}

// Implementation for Mutex
impl<T: Send + Sync + 'static> Applicative<T> for Mutex<T> {
  type ApplicativeSelf<U: Send + Sync + 'static> = Mutex<U>;

  fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static,
  {
    Mutex::new(value)
  }

  fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    let a = self.into_inner().unwrap();
    let mut func = f.into_inner().unwrap();
    Mutex::new(func(a))
  }
}

// Implementation for RwLock
impl<T: Send + Sync + 'static> Applicative<T> for RwLock<T> {
  type ApplicativeSelf<U: Send + Sync + 'static> = RwLock<U>;

  fn pure<U>(value: U) -> Self::ApplicativeSelf<U>
  where
    U: Send + Sync + 'static,
  {
    RwLock::new(value)
  }

  fn ap<U, F>(self, f: Self::ApplicativeSelf<F>) -> Self::ApplicativeSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    let a = self.into_inner().unwrap();
    let mut func = f.into_inner().unwrap();
    RwLock::new(func(a))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::thread;

  // Define test functions that implement Debug
  const FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
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
      fn test_option_applicative_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = Some(x).ap(Some(id));
          assert_eq!(result, Some(x));
      }

      #[test]
      fn test_option_applicative_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Some(x).ap(Some(f)).ap(Some(g));
          let result2 = Some(x).ap(Some(|x| g(f(x))));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_vec_applicative_identity(xs in prop::collection::vec(any::<i32>(), 0..100)) {
          let id = |x: i32| x;
          let result = xs.clone().ap(vec![id]);
          assert_eq!(result, xs);
      }

      #[test]
      fn test_vec_applicative_composition(
          xs in prop::collection::vec(any::<i32>(), 0..100),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = xs.clone().ap(vec![f]).ap(vec![g]);
          let result2 = xs.ap(vec![|x| g(f(x))]);
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_result_applicative_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result: Result<i32, &str> = Ok(x).ap(Ok(id));
          assert_eq!(result, Ok(x));
      }

      #[test]
      fn test_result_applicative_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1: Result<i32, &str> = Ok(x).ap(Ok(f)).ap(Ok(g));
          let result2 = Ok(x).ap(Ok(|x| g(f(x))));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_mutex_applicative_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = Mutex::new(x).ap(Mutex::new(id));
          assert_eq!(*result.lock().unwrap(), x);
      }

      #[test]
      fn test_mutex_applicative_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Mutex::new(x).ap(Mutex::new(f)).ap(Mutex::new(g));
          let result2 = Mutex::new(x).ap(Mutex::new(|x| g(f(x))));
          assert_eq!(*result1.lock().unwrap(), *result2.lock().unwrap());
      }

      #[test]
      fn test_rwlock_applicative_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = RwLock::new(x).ap(RwLock::new(id));
          assert_eq!(*result.read().unwrap(), x);
      }

      #[test]
      fn test_rwlock_applicative_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = RwLock::new(x).ap(RwLock::new(f)).ap(RwLock::new(g));
          let result2 = RwLock::new(x).ap(RwLock::new(|x| g(f(x))));
          assert_eq!(*result1.read().unwrap(), *result2.read().unwrap());
      }

      #[test]
      fn test_thread_safety_properties(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = test_thread_safety(|x| Some(x).ap(Some(f)), x);
          assert_eq!(result, Some(f(x)));

          let xs = vec![x];
          let result = test_thread_safety(|xs| xs.ap(vec![f]), xs);
          assert_eq!(result, vec![f(x)]);

          let r: Result<i32, &str> = Ok(x);
          let result = test_thread_safety(|r| r.ap(Ok(f)), r);
          assert_eq!(result, Ok(f(x)));
      }
  }
}
