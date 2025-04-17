//! Filterable trait and implementations.
//!
//! A filterable is a type that can be filtered based on a predicate.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, RwLock};

/// The Filterable trait defines the basic operations for filterable types.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The predicate function F must implement Send + Sync + 'static
pub trait Filterable<T: Send + Sync + 'static> {
  /// The higher-kinded type that results from filtering this filterable
  type HigherSelf<U: Send + Sync + 'static>: Filterable<U>;

  /// Filters elements based on a predicate
  fn filter<F>(self, f: F) -> Self::HigherSelf<T>
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Filterable<T> for Option<T> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn filter<F>(self, f: F) -> Self::HigherSelf<T>
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static,
  {
    self.filter(f)
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Filterable<T> for Vec<T> {
  type HigherSelf<U: Send + Sync + 'static> = Vec<U>;

  fn filter<F>(self, f: F) -> Self::HigherSelf<T>
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static,
  {
    self.into_iter().filter(f).collect()
  }
}

// Implementation for HashMap (values only)
impl<K: Eq + Hash + Send + Sync + 'static, V: Send + Sync + 'static> Filterable<V>
  for HashMap<K, V>
{
  type HigherSelf<U: Send + Sync + 'static> = HashMap<K, U>;

  fn filter<F>(self, mut f: F) -> Self::HigherSelf<V>
  where
    F: FnMut(&V) -> bool + Send + Sync + 'static,
  {
    self.into_iter().filter(|(_, v)| f(v)).collect()
  }
}

// Implementation for Mutex
impl<T: Send + Sync + 'static> Filterable<T> for Mutex<T> {
  type HigherSelf<U: Send + Sync + 'static> = Mutex<U>;

  fn filter<F>(self, mut f: F) -> Self::HigherSelf<T>
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static,
  {
    let value = self.into_inner().unwrap();
    if f(&value) {
      Mutex::new(value)
    } else {
      Mutex::new(value)
    }
  }
}

// Implementation for RwLock
impl<T: Send + Sync + 'static> Filterable<T> for RwLock<T> {
  type HigherSelf<U: Send + Sync + 'static> = RwLock<U>;

  fn filter<F>(self, mut f: F) -> Self::HigherSelf<T>
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static,
  {
    let value = self.into_inner().unwrap();
    if f(&value) {
      RwLock::new(value)
    } else {
      RwLock::new(value)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test predicates
  const PREDICATES: &[fn(&i32) -> bool] = &[
    |x| x > &0,
    |x| x % 2 == 0,
    |x| x < &100,
    |x| x.abs() > 10,
    |x| x % 3 == 0,
  ];

  // Helper function to test thread safety
  fn test_thread_safety<T: Filterable<i32> + Send + Sync + 'static>(value: T, predicate_idx: usize)
  where
    T::HigherSelf<i32>: Send + Sync + 'static,
  {
    let predicate = PREDICATES[predicate_idx];
    let handle = std::thread::spawn(move || value.filter(predicate));
    assert!(handle.join().is_ok());
  }

  proptest! {
    #[test]
    fn test_option_filter(
      x in any::<i32>(),
      predicate_idx in 0..PREDICATES.len()
    ) {
      let predicate = PREDICATES[predicate_idx];
      let option = Some(x);
      let filtered = option.filter(predicate);
      assert_eq!(filtered, if predicate(&x) { Some(x) } else { None });
    }

    #[test]
    fn test_vec_filter(
      xs in prop::collection::vec(any::<i32>(), 0..100),
      predicate_idx in 0..PREDICATES.len()
    ) {
      let predicate = PREDICATES[predicate_idx];
      let filtered = xs.filter(predicate);
      assert_eq!(filtered, xs.into_iter().filter(|x| predicate(x)).collect::<Vec<_>>());
    }

    #[test]
    fn test_hashmap_filter(
      entries in prop::collection::hash_map(any::<i32>(), any::<i32>(), 0..10),
      predicate_idx in 0..PREDICATES.len()
    ) {
      let predicate = PREDICATES[predicate_idx];
      let filtered = entries.filter(predicate);
      assert_eq!(
        filtered,
        entries.into_iter()
          .filter(|(_, v)| predicate(v))
          .collect::<HashMap<_, _>>()
      );
    }

    #[test]
    fn test_mutex_filter(
      x in any::<i32>(),
      predicate_idx in 0..PREDICATES.len()
    ) {
      let predicate = PREDICATES[predicate_idx];
      let mutex = Mutex::new(x);
      let filtered = mutex.filter(predicate);
      let value = filtered.into_inner().unwrap();
      assert_eq!(value, x);
    }

    #[test]
    fn test_rwlock_filter(
      x in any::<i32>(),
      predicate_idx in 0..PREDICATES.len()
    ) {
      let predicate = PREDICATES[predicate_idx];
      let rwlock = RwLock::new(x);
      let filtered = rwlock.filter(predicate);
      let value = filtered.into_inner().unwrap();
      assert_eq!(value, x);
    }

    #[test]
    fn test_thread_safety_option(
      x in any::<i32>(),
      predicate_idx in 0..PREDICATES.len()
    ) {
      test_thread_safety(Some(x), predicate_idx);
    }

    #[test]
    fn test_thread_safety_vec(
      xs in prop::collection::vec(any::<i32>(), 0..100),
      predicate_idx in 0..PREDICATES.len()
    ) {
      test_thread_safety(xs, predicate_idx);
    }

    #[test]
    fn test_thread_safety_hashmap(
      entries in prop::collection::hash_map(any::<i32>(), any::<i32>(), 0..10),
      predicate_idx in 0..PREDICATES.len()
    ) {
      test_thread_safety(entries, predicate_idx);
    }

    #[test]
    fn test_thread_safety_mutex(
      x in any::<i32>(),
      predicate_idx in 0..PREDICATES.len()
    ) {
      test_thread_safety(Mutex::new(x), predicate_idx);
    }

    #[test]
    fn test_thread_safety_rwlock(
      x in any::<i32>(),
      predicate_idx in 0..PREDICATES.len()
    ) {
      test_thread_safety(RwLock::new(x), predicate_idx);
    }
  }
}
