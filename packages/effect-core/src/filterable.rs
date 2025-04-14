//! Filterable trait and implementations.
//!
//! A filterable is a type that can be filtered based on a predicate.

use std::collections::HashMap;
use std::hash::Hash;

/// The Filterable trait defines the basic operations for filterable types.
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
impl<K: Send + Sync + 'static + Eq + Hash, V: Send + Sync + 'static> Filterable<V>
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

#[cfg(test)]
mod tests {
  use super::*;

  // Define a simple filterable for testing
  #[derive(Debug, PartialEq, Clone)]
  struct TestFilterable<T>(T);

  impl<T> TestFilterable<T> {
    fn new(value: T) -> Self {
      TestFilterable(value)
    }
  }

  impl<T: Send + Sync + 'static> Filterable<T> for TestFilterable<T> {
    type HigherSelf<U: Send + Sync + 'static> = TestFilterable<U>;

    fn filter<F>(self, mut f: F) -> Self::HigherSelf<T>
    where
      F: FnMut(&T) -> bool + Send + Sync + 'static,
    {
      if f(&self.0) {
        self
      } else {
        panic!("TestFilterable cannot be empty")
      }
    }
  }

  // Test filterable laws
  #[test]
  fn test_filterable_laws() {
    // Identity law: filter (const True) = id
    let f = TestFilterable::new(42);
    let always_true = |_: &i32| true;
    assert_eq!(f.clone().filter(always_true), f);

    // Composition law: filter p . filter q = filter (p && q)
    let f = TestFilterable::new(42);
    let p = |x: &i32| *x > 0;
    let q = |x: &i32| *x < 100;
    let lhs = f.clone().filter(p).filter(q);
    let rhs = f.filter(move |x| p(x) && q(x));
    assert_eq!(lhs, rhs);
  }

  // Test basic operations
  #[test]
  fn test_filter() {
    let f = TestFilterable::new(42);
    let filtered = f.filter(|x| *x > 0);
    assert_eq!(filtered, TestFilterable(42));
  }

  // Test type conversions
  #[test]
  fn test_type_conversions() {
    let f = TestFilterable::new(42);
    let filtered = f.filter(|x| *x > 0);
    assert_eq!(filtered, TestFilterable(42));
  }

  // Test composition
  #[test]
  fn test_composition() {
    let f = TestFilterable::new(42);
    let filtered = f.clone().filter(|x| *x > 0).filter(|x| *x < 100);
    assert_eq!(filtered, TestFilterable(42));
  }

  mod option_tests {
    #[test]
    fn test_some_filter_true() {
      let some = Some(42);
      let filtered = some.filter(|x| *x > 0);
      assert_eq!(filtered, Some(42));
    }

    #[test]
    fn test_some_filter_false() {
      let some = Some(42);
      let filtered = some.filter(|x| *x < 0);
      assert_eq!(filtered, None);
    }

    #[test]
    fn test_none_filter() {
      let none: Option<i32> = None;
      let filtered = none.filter(|x| *x > 0);
      assert_eq!(filtered, None);
    }

    #[test]
    fn test_type_conversion() {
      let some = Some(42);
      let filtered = some.filter(|x| *x > 0);
      assert_eq!(filtered, Some(42));
    }

    #[test]
    fn test_composition() {
      let some = Some(42);
      let filtered = some.filter(|x| *x > 0).filter(|x| *x < 100);
      assert_eq!(filtered, Some(42));
    }
  }

  mod vec_tests {
    use super::*;

    #[test]
    fn test_empty_vec() {
      let empty: Vec<i32> = vec![];
      let filtered = empty.filter(|x| *x > 0);
      let expected: Vec<i32> = vec![];
      assert_eq!(filtered, expected);
    }

    #[test]
    fn test_filter_all() {
      let vec = vec![1, 2, 3, 4, 5];
      let filtered = vec.filter(|x| *x > 0);
      assert_eq!(filtered, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_filter_some() {
      let vec = vec![1, 2, 3, 4, 5];
      let filtered = vec.filter(|x| *x > 3);
      assert_eq!(filtered, vec![4, 5]);
    }

    #[test]
    fn test_filter_none() {
      let vec = vec![1, 2, 3, 4, 5];
      let filtered = vec.filter(|x| *x > 10);
      let expected: Vec<i32> = vec![];
      assert_eq!(filtered, expected);
    }

    #[test]
    fn test_type_conversion() {
      let vec = vec![1, 2, 3, 4, 5];
      let filtered = vec.filter(|x| *x > 0);
      assert_eq!(filtered, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_composition() {
      let vec = vec![1, 2, 3, 4, 5];
      let filtered = vec.filter(|x| *x > 0).filter(|x| *x < 4);
      assert_eq!(filtered, vec![1, 2, 3]);
    }
  }

  mod hashmap_tests {
    use super::*;

    #[test]
    fn test_filter_values() {
      let mut map = HashMap::new();
      map.insert("a", 1);
      map.insert("b", 2);
      map.insert("c", 3);
      let filtered = map.filter(|v| *v > 1);
      assert_eq!(filtered.len(), 2);
      assert!(filtered.contains_key("b"));
      assert!(filtered.contains_key("c"));
    }

    #[test]
    fn test_filter_empty() {
      let map: HashMap<&str, i32> = HashMap::new();
      let filtered = map.filter(|v| *v > 0);
      assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_all() {
      let mut map = HashMap::new();
      map.insert("a", 1);
      map.insert("b", 2);
      let filtered = map.filter(|v| *v > 0);
      assert_eq!(filtered.len(), 2);
      assert!(filtered.contains_key("a"));
      assert!(filtered.contains_key("b"));
    }

    #[test]
    fn test_type_conversion() {
      let mut map = HashMap::new();
      map.insert("a", 1);
      map.insert("b", 2);
      let filtered = map.filter(|v| *v > 0);
      assert_eq!(filtered.len(), 2);
      assert!(filtered.contains_key("a"));
      assert!(filtered.contains_key("b"));
    }

    #[test]
    fn test_composition() {
      let mut map = HashMap::new();
      map.insert("a", 1);
      map.insert("b", 2);
      map.insert("c", 3);
      let filtered = map.filter(|v| *v > 0).filter(|v| *v < 3);
      assert_eq!(filtered.len(), 2);
      assert!(filtered.contains_key("a"));
      assert!(filtered.contains_key("b"));
    }
  }
}
