//! Distinctable trait and implementations.
//!
//! A distinctable is a type that can remove duplicate elements while preserving order.

use std::collections::HashSet;
use std::hash::Hash;

/// A trait for types that can remove duplicate elements.
pub trait Distinctable {
  type HigherSelf;

  /// Returns a new collection with duplicate elements removed.
  fn distinct(self) -> Self
  where
    Self::HigherSelf: Eq + Hash + Clone;

  /// Returns a new collection with duplicate elements removed based on a key function.
  fn distinct_by<F, K>(self, f: F) -> Self
  where
    F: Fn(&Self::HigherSelf) -> K,
    K: Eq + Hash,
    Self::HigherSelf: Clone;
}

// Implementation for Vec
impl<T: Send + Sync + Clone> Distinctable for Vec<T> {
  type HigherSelf = T;

  fn distinct(self) -> Self
  where
    T: Eq + Hash + Clone,
  {
    let mut seen = HashSet::new();
    self
      .into_iter()
      .filter(|x| seen.insert(x.clone()))
      .collect()
  }

  fn distinct_by<F, K>(self, f: F) -> Self
  where
    F: Fn(&T) -> K,
    K: Eq + Hash,
    T: Clone,
  {
    let mut seen = HashSet::new();
    self.into_iter().filter(|x| seen.insert(f(x))).collect()
  }
}

// Implementation for Option
impl<T: Clone> Distinctable for Option<T> {
  type HigherSelf = T;

  fn distinct(self) -> Self
  where
    T: Eq + Hash + Clone,
  {
    self
  }

  fn distinct_by<F, K>(self, f: F) -> Self
  where
    F: Fn(&T) -> K,
    K: Eq + Hash,
    T: Clone,
  {
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_distinct() {
    let v = vec![1, 2, 2, 3, 3, 3, 4];
    let distinct = v.distinct();
    assert_eq!(distinct, vec![1, 2, 3, 4]);
  }

  #[test]
  fn test_vec_distinct_by() {
    let v = vec!["hello", "world", "rust", "hello", "world"];
    let distinct = v.distinct_by(|s| s.len());
    assert_eq!(distinct, vec!["hello", "rust"]);
  }

  #[test]
  fn test_option_distinct() {
    let opt = Some(42);
    assert_eq!(opt.distinct(), Some(42));
    assert_eq!(None::<i32>.distinct(), None);
  }

  #[test]
  fn test_option_distinct_by() {
    let opt = Some("hello");
    assert_eq!(opt.distinct_by(|s| s.len()), Some("hello"));
    assert_eq!(None::<&str>.distinct_by(|s| s.len()), None);
  }
}
