//! Functor trait and implementations.
//!
//! A functor is a type that can be mapped over, preserving structure.
//!

use super::category::Category;
use std::sync::{Arc, Mutex, RwLock};

/// A functor is a type that can be mapped over.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The higher-kinded type HigherSelf<U> must implement Send + Sync + 'static
/// - The mapping function F must implement Send + Sync + 'static
/// - The mapped type U must implement Send + Sync + 'static
pub trait Functor<T: Send + Sync + 'static>: Category<T, T> {
  /// The higher-kinded type that results from mapping over this functor
  type HigherSelf<U: Send + Sync + 'static>: Send + Sync + 'static;

  /// Maps a function over the functor
  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Functor<T> for Option<T> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    self.map(f)
  }
}

impl<T: Send + Sync + 'static> Category<T, T> for Option<T> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Option<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Some(())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    f.and_then(|_| g)
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Some(())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    f.map(|_| ())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    f.map(|_| ())
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Functor<T> for Vec<T> {
  type HigherSelf<U: Send + Sync + 'static> = Vec<U>;

  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    self.into_iter().map(f).collect()
  }
}

impl<T: Send + Sync + 'static> Category<T, T> for Vec<T> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Vec<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    vec![]
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    f.into_iter().chain(g.into_iter()).collect()
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    vec![]
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    f.into_iter().map(|_| ()).collect()
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    f.into_iter().map(|_| ()).collect()
  }
}

// Implementation for Result
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Functor<T> for Result<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = Result<U, E>;

  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    self.map(f)
  }
}

impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Category<T, T> for Result<T, E> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Result<B, E>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Ok(())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    f.and_then(|_| g)
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Ok(())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    f.map(|_| ())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    f.map(|_| ())
  }
}

// Implementation for Box
impl<T: Send + Sync + 'static> Functor<T> for Box<T> {
  type HigherSelf<U: Send + Sync + 'static> = Box<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    Box::new(f(*self))
  }
}

impl<T: Send + Sync + 'static> Category<T, T> for Box<T> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Box<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Box::new(())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Box::new(())
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Box::new(())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Box::new(())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Box::new(())
  }
}

// Implementation for Arc
impl<T: Send + Sync + 'static> Functor<T> for Arc<T> {
  type HigherSelf<U: Send + Sync + 'static> = Arc<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    Arc::new(f((*self).clone()))
  }
}

impl<T: Send + Sync + 'static> Category<T, T> for Arc<T> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Arc<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Arc::new(())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Arc::new(())
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Arc::new(())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Arc::new(())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Arc::new(())
  }
}

// Implementation for Mutex
impl<T: Send + Sync + 'static> Functor<T> for Mutex<T> {
  type HigherSelf<U: Send + Sync + 'static> = Mutex<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    Mutex::new(f(self.into_inner().unwrap()))
  }
}

impl<T: Send + Sync + 'static> Category<T, T> for Mutex<T> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Mutex<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Mutex::new(())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Mutex::new(())
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Mutex::new(())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Mutex::new(())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Mutex::new(())
  }
}

// Implementation for RwLock
impl<T: Send + Sync + 'static> Functor<T> for RwLock<T> {
  type HigherSelf<U: Send + Sync + 'static> = RwLock<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    RwLock::new(f(self.into_inner().unwrap()))
  }
}

impl<T: Send + Sync + 'static> Category<T, T> for RwLock<T> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = RwLock<B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    RwLock::new(())
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    RwLock::new(())
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    RwLock::new(())
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    RwLock::new(())
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    RwLock::new(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::sync::Arc;
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
      fn test_option_functor_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = Some(x).map(id);
          assert_eq!(result, Some(x));
      }

      #[test]
      fn test_option_functor_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Some(x).map(f).map(g);
          let result2 = Some(x).map(|x| g(f(x)));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_vec_functor_identity(xs in prop::collection::vec(any::<i32>(), 0..100)) {
          let id = |x: i32| x;
          let result = xs.clone().map(id);
          assert_eq!(result, xs);
      }

      #[test]
      fn test_vec_functor_composition(
          xs in prop::collection::vec(any::<i32>(), 0..100),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = xs.clone().map(f).map(g);
          let result2 = xs.map(|x| g(f(x)));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_result_functor_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result: Result<i32, &str> = Ok(x).map(id);
          assert_eq!(result, Ok(x));
      }

      #[test]
      fn test_result_functor_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1: Result<i32, &str> = Ok(x).map(f).map(g);
          let result2 = Ok(x).map(|x| g(f(x)));
          assert_eq!(result1, result2);
      }

      #[test]
      fn test_box_functor_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = Box::new(x).map(id);
          assert_eq!(*result, x);
      }

      #[test]
      fn test_box_functor_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Box::new(x).map(f).map(g);
          let result2 = Box::new(x).map(|x| g(f(x)));
          assert_eq!(*result1, *result2);
      }

      #[test]
      fn test_arc_functor_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = Arc::new(x).map(id);
          assert_eq!(*result, x);
      }

      #[test]
      fn test_arc_functor_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Arc::new(x).map(f).map(g);
          let result2 = Arc::new(x).map(|x| g(f(x)));
          assert_eq!(*result1, *result2);
      }

      #[test]
      fn test_mutex_functor_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = Mutex::new(x).map(id);
          assert_eq!(*result.lock().unwrap(), x);
      }

      #[test]
      fn test_mutex_functor_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = Mutex::new(x).map(f).map(g);
          let result2 = Mutex::new(x).map(|x| g(f(x)));
          assert_eq!(*result1.lock().unwrap(), *result2.lock().unwrap());
      }

      #[test]
      fn test_rwlock_functor_identity(x in any::<i32>()) {
          let id = |x: i32| x;
          let result = RwLock::new(x).map(id);
          assert_eq!(*result.read().unwrap(), x);
      }

      #[test]
      fn test_rwlock_functor_composition(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len(),
          g_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let g = FUNCTIONS[g_idx];
          let result1 = RwLock::new(x).map(f).map(g);
          let result2 = RwLock::new(x).map(|x| g(f(x)));
          assert_eq!(*result1.read().unwrap(), *result2.read().unwrap());
      }

      #[test]
      fn test_thread_safety_properties(
          x in any::<i32>(),
          f_idx in 0..FUNCTIONS.len()
      ) {
          let f = FUNCTIONS[f_idx];
          let result = test_thread_safety(|x| Some(x).map(f), x);
          assert_eq!(result, Some(f(x)));

          let xs = vec![x];
          let result = test_thread_safety(|xs| xs.map(f), xs);
          assert_eq!(result, vec![f(x)]);

          let r: Result<i32, &str> = Ok(x);
          let result = test_thread_safety(|r| r.map(f), r);
          assert_eq!(result, Ok(f(x)));
      }
  }
}
