// This file contains a commented out implementation of Foldable for HashMap
// It requires a Functor implementation first
// To be uncommented once the Functor implementation is added

/*
use crate::traits::foldable::Foldable;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;

impl<K, V>
  Foldable<V>
  for HashMap<K, V>
where
  K: Eq + std::hash::Hash + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    F: FnMut(A, &V) -> A + CloneableThreadSafe,
    A: CloneableThreadSafe,
  {
    self.into_iter().fold(init, |acc, (_, v)| f(acc, &v))
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    F: FnMut(&V, A) -> A + CloneableThreadSafe,
    A: CloneableThreadSafe,
  {
    let mut values: Vec<V> = self.into_iter().map(|(_, v)| v).collect();
    values.reverse();
    values.iter().fold(init, |acc, v| f(v, acc))
  }

  fn reduce<F>(self, mut f: F) -> Option<V>
  where
    F: FnMut(&V, &V) -> V + CloneableThreadSafe,
  {
    let mut values: Vec<V> = self.into_iter().map(|(_, v)| v).collect();
    if values.is_empty() {
      return None;
    }

    let first = values.remove(0);
    Some(values.iter().fold(first, |acc, v| f(&acc, v)))
  }

  fn reduce_right<F>(self, mut f: F) -> Option<V>
  where
    F: FnMut(&V, &V) -> V + CloneableThreadSafe,
  {
    let mut values: Vec<V> = self.into_iter().map(|(_, v)| v).collect();
    if values.is_empty() {
      return None;
    }

    values.reverse();
    let first = values.remove(0);
    Some(values.iter().fold(first, |acc, v| f(v, &acc)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::collections::HashMap;

  #[test]
  fn test_fold() {
    let mut map = HashMap::new();
    map.insert("one", 1);
    map.insert("two", 2);
    map.insert("three", 3);

    let sum = Foldable::fold(map, 0, |acc, v| acc + v);
    assert_eq!(sum, 6);
  }

  #[test]
  fn test_fold_right() {
    let mut map = HashMap::new();
    map.insert("one", 1);
    map.insert("two", 2);
    map.insert("three", 3);

    let sum = Foldable::fold_right(map, 0, |v, acc| acc + v);
    assert_eq!(sum, 6);
  }

  #[test]
  fn test_reduce() {
    let mut map = HashMap::new();
    map.insert("one", 1);
    map.insert("two", 2);
    map.insert("three", 3);

    let sum = Foldable::reduce(map, |a, b| a + b).unwrap();
    assert_eq!(sum, 6);
  }

  #[test]
  fn test_reduce_right() {
    let mut map = HashMap::new();
    map.insert("one", 1);
    map.insert("two", 2);
    map.insert("three", 3);

    let sum = Foldable::reduce_right(map, |a, b| a + b).unwrap();
    assert_eq!(sum, 6);
  }

  proptest! {
    #[test]
    fn prop_fold_sum(entries in proptest::collection::hash_map(
      proptest::string::string_regex("[a-z]{1,10}").unwrap(),
      0i8..10,
      0..10
    )) {
      let map = entries.clone();

      // Calculate expected sum directly
      let expected_sum: i32 = entries.values().map(|&v| v as i32).sum();

      // Calculate sum using fold
      let actual_sum = Foldable::fold(map, 0i32, |acc: i32, &v| acc + v as i32);

      // Verify results
      prop_assert_eq!(actual_sum, expected_sum);
    }

    #[test]
    fn prop_fold_right_sum(entries in proptest::collection::hash_map(
      proptest::string::string_regex("[a-z]{1,10}").unwrap(),
      0i8..10,
      0..10
    )) {
      let map = entries.clone();

      // Calculate expected sum directly
      let expected_sum: i32 = entries.values().map(|&v| v as i32).sum();

      // Calculate sum using fold_right
      let actual_sum = Foldable::fold_right(map, 0i32, |&v, acc: i32| acc + v as i32);

      // Verify results
      prop_assert_eq!(actual_sum, expected_sum);
    }
  }
}
*/
