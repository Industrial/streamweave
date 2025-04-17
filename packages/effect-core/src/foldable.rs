//! Foldable trait and implementations.
//!
//! A foldable is a functor that can be folded over to reduce its values.

use crate::functor::Functor;
use std::sync::{Mutex, RwLock};

/// A foldable is a functor that can be folded over to reduce its values.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The accumulator type A must implement Send + Sync + 'static
/// - The folding function F must implement Send + Sync + 'static
pub trait Foldable<T: Send + Sync + 'static>: Functor<T> {
  /// Folds over the foldable using a binary operation and an initial value
  fn fold<A, F>(self, init: A, f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(A, T) -> A + Send + Sync + 'static;

  /// Folds over the foldable from the right using a binary operation and an initial value
  fn fold_right<A, F>(self, init: A, f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(T, A) -> A + Send + Sync + 'static;

  /// Reduces the foldable using a binary operation
  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static;

  /// Reduces the foldable from the right using a binary operation
  fn reduce_right<F>(self, f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Foldable<T> for Option<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(A, T) -> A + Send + Sync + 'static,
  {
    match self {
      Some(x) => f(init, x),
      None => init,
    }
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(T, A) -> A + Send + Sync + 'static,
  {
    match self {
      Some(x) => f(x, init),
      None => init,
    }
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    self
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    self
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Foldable<T> for Vec<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(A, T) -> A + Send + Sync + 'static,
  {
    self.into_iter().fold(init, f)
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(T, A) -> A + Send + Sync + 'static,
  {
    self.into_iter().rev().fold(init, |acc, x| f(x, acc))
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    let mut iter = self.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, f))
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    let mut iter = self.into_iter().rev();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, x| f(x, acc)))
  }
}

// Implementation for Result
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Foldable<T> for Result<T, E> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(A, T) -> A + Send + Sync + 'static,
  {
    match self {
      Ok(x) => f(init, x),
      Err(_) => init,
    }
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(T, A) -> A + Send + Sync + 'static,
  {
    match self {
      Ok(x) => f(x, init),
      Err(_) => init,
    }
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    self.ok()
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    self.ok()
  }
}

// Implementation for Mutex
impl<T: Send + Sync + 'static> Foldable<T> for Mutex<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(A, T) -> A + Send + Sync + 'static,
  {
    f(init, self.into_inner().unwrap())
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(T, A) -> A + Send + Sync + 'static,
  {
    f(self.into_inner().unwrap(), init)
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    Some(self.into_inner().unwrap())
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    Some(self.into_inner().unwrap())
  }
}

// Implementation for RwLock
impl<T: Send + Sync + 'static> Foldable<T> for RwLock<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(A, T) -> A + Send + Sync + 'static,
  {
    f(init, self.into_inner().unwrap())
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: Send + Sync + 'static,
    F: FnMut(T, A) -> A + Send + Sync + 'static,
  {
    f(self.into_inner().unwrap(), init)
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    Some(self.into_inner().unwrap())
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: FnMut(T, T) -> T + Send + Sync + 'static,
  {
    Some(self.into_inner().unwrap())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::fmt::Debug;
  use std::sync::mpsc;
  use std::sync::Arc;
  use std::thread;

  // Define test functions that implement Debug
  const FUNCTIONS: &[fn(i32, i32) -> i32] = &[
    |x, y| x + y,
    |x, y| x * y,
    |x, y| x - y,
    |x, y| x.max(y),
    |x, y| x.min(y),
    |x, y| x.abs_diff(y).try_into().unwrap(),
  ];

  // Test helper for thread safety
  fn test_thread_safety<T, F, R>(f: F, x: T) -> R
  where
    T: Send + Sync + 'static,
    F: Fn(T) -> R + Send + Sync + 'static,
    R: Send + Sync + 'static + PartialEq + Debug,
  {
    let (tx, rx) = mpsc::channel();
    let f = Arc::new(f);
    let x = Arc::new(x);

    let handle = thread::spawn(move || {
      let result = f((*x).clone());
      tx.send(result).unwrap()
    });

    handle.join().unwrap();
    rx.recv().unwrap()
  }

  proptest! {
      #[test]
      fn test_option_fold(
          x in any::<Option<i32>>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = x.clone().fold(init, |acc, x| f(acc, x));
          let expected = match x {
              Some(x) => f(init, x),
              None => init,
          };
          assert_eq!(result, expected);
      }

      #[test]
      fn test_option_fold_right(
          x in any::<Option<i32>>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = x.clone().fold_right(init, |x, acc| f(x, acc));
          let expected = match x {
              Some(x) => f(x, init),
              None => init,
          };
          assert_eq!(result, expected);
      }

      #[test]
      fn test_vec_fold(
          xs in prop::collection::vec(any::<i32>(), 0..100),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = xs.clone().fold(init, |acc, x| f(acc, x));
          let expected = xs.into_iter().fold(init, |acc, x| f(acc, x));
          assert_eq!(result, expected);
      }

      #[test]
      fn test_vec_fold_right(
          xs in prop::collection::vec(any::<i32>(), 0..100),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = xs.clone().fold_right(init, |x, acc| f(x, acc));
          let expected = xs.into_iter().rev().fold(init, |acc, x| f(x, acc));
          assert_eq!(result, expected);
      }

      #[test]
      fn test_result_fold(
          x in any::<Result<i32, &str>>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = x.clone().fold(init, |acc, x| f(acc, x));
          let expected = match x {
              Ok(x) => f(init, x),
              Err(_) => init,
          };
          assert_eq!(result, expected);
      }

      #[test]
      fn test_result_fold_right(
          x in any::<Result<i32, &str>>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = x.clone().fold_right(init, |x, acc| f(x, acc));
          let expected = match x {
              Ok(x) => f(x, init),
              Err(_) => init,
          };
          assert_eq!(result, expected);
      }

      #[test]
      fn test_mutex_fold(
          x in any::<i32>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = Mutex::new(x).fold(init, |acc, x| f(acc, x));
          assert_eq!(result, f(init, x));
      }

      #[test]
      fn test_mutex_fold_right(
          x in any::<i32>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = Mutex::new(x).fold_right(init, |x, acc| f(x, acc));
          assert_eq!(result, f(x, init));
      }

      #[test]
      fn test_rwlock_fold(
          x in any::<i32>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = RwLock::new(x).fold(init, |acc, x| f(acc, x));
          assert_eq!(result, f(init, x));
      }

      #[test]
      fn test_rwlock_fold_right(
          x in any::<i32>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = RwLock::new(x).fold_right(init, |x, acc| f(x, acc));
          assert_eq!(result, f(x, init));
      }

      #[test]
      fn test_thread_safety_properties(
          x in any::<i32>(),
          init in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];

          // Test Option
          let result = test_thread_safety(|x| Some(x).fold(init, |acc, x| f(acc, x)), x);
          assert_eq!(result, f(init, x));

          // Test Vec
          let xs = vec![x];
          let result = test_thread_safety(|xs| xs.fold(init, |acc, x| f(acc, x)), xs);
          assert_eq!(result, f(init, x));

          // Test Result - Ok case
          let ok_result: Result<i32, &str> = Ok(x);
          let result = test_thread_safety(|r: Result<i32, &str>| r.fold(init, |acc, x| f(acc, x)), ok_result);
          assert_eq!(result, Ok(f(init, x)));

          // Test Result - Err case
          let err_result: Result<i32, &str> = Err("error");
          let result = test_thread_safety(|r: Result<i32, &str>| r.fold(init, |acc, x| f(acc, x)), err_result);
          assert_eq!(result, Err("error"));

          // Test Mutex
          let m = Mutex::new(x);
          let result = test_thread_safety(|m| m.fold(init, |acc, x| f(acc, x)), m);
          assert_eq!(result, f(init, x));

          // Test RwLock
          let rw = RwLock::new(x);
          let result = test_thread_safety(|rw| rw.fold(init, |acc, x| f(acc, x)), rw);
          assert_eq!(result, f(init, x));
      }
  }
}
