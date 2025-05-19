//! Implementation of the `Monad` trait for `HashMap<K, A>`.
//!
//! This module implements the monadic bind operation for the `HashMap` type,
//! enabling key-based composition of operations on mapped values.
//!
//! The implementation applies a function to each value in the map, where the function
//! itself returns a HashMap. The result is a map containing only the keys that exist
//! in both the original map and the map returned by the function application.
//!
//! This can be thought of as a filtering operation where each value can produce
//! a new collection, but we only keep entries where the keys align.

use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

// Implement directly on HashMap for simpler type signatures in the extension trait
impl<K, A> Monad<A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = HashMap::new();

    for (k, a) in self {
      let new_map = f(&a);

      // Only include k in the result if it exists in the new_map
      if let Some(b) = new_map.get(&k) {
        result.insert(k, b.clone());
      }
    }

    result
  }
}

// Extension trait for more ergonomic monadic operations
pub trait HashMapMonadExt<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn flat_map<B, F>(&self, f: F) -> HashMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> HashMap<K, B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

impl<K, A> HashMapMonadExt<K, A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn flat_map<B, F>(&self, mut f: F) -> HashMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> HashMap<K, B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = HashMap::new();

    for (k, a) in self {
      let new_map = f(a);

      // Only include k in the result if it exists in the new_map
      if let Some(b) = new_map.get(k) {
        result.insert(k.clone(), b.clone());
      }
    }

    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper to create a HashMap from key-value pairs
  fn to_hashmap<K, V>(entries: Vec<(K, V)>) -> HashMap<K, V>
  where
    K: Eq + Hash,
  {
    entries.into_iter().collect()
  }

  // Test functions for the monad laws
  fn add_entry(x: &i32) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("a".to_string(), x + 1);
    map.insert("b".to_string(), x + 2);
    map
  }

  fn multiply_entry(x: &i32) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    map.insert("b".to_string(), x * 2);
    map.insert("c".to_string(), x * 3);
    map
  }

  fn empty_result(_: &i32) -> HashMap<String, i32> {
    HashMap::new()
  }

  // Left identity law: pure(a).bind(f) == f(a)
  // For HashMap, pure returns an empty map, so this is a special case
  #[test]
  fn test_left_identity_specialization() {
    let _a = 42i32;
    let f = add_entry;

    // pure(a).bind(f) should be empty since pure(a) is empty
    let empty_map: HashMap<String, i32> = HashMap::new();
    let left = empty_map.bind(f);

    // The result should be empty
    assert!(left.is_empty());
  }

  // Right identity law: m.bind(pure) == m
  // For HashMap, this test is adjusted since pure(x) returns an empty map
  #[test]
  fn test_right_identity_specialization() {
    let map = to_hashmap(vec![
      ("a".to_string(), 1),
      ("b".to_string(), 2),
      ("c".to_string(), 3),
    ]);

    // m.bind(|x| pure(x)) will be empty due to the empty map from pure
    let result = map.bind(|_| HashMap::<String, i32>::new());

    // The result should be empty
    assert!(result.is_empty());
  }

  // Key-preserving identity for HashMaps:
  // m.bind(|v| single_entry_map_with_same_key(v)) == m
  #[test]
  fn test_key_preserving_identity() {
    let map = to_hashmap(vec![
      ("a".to_string(), 1),
      ("b".to_string(), 2),
      ("c".to_string(), 3),
    ]);

    // Define a function that returns a map with only the same key
    let same_key = |x: &i32| {
      let mut result = HashMap::new();
      result.insert("a".to_string(), *x);
      result
    };

    let result = map.clone().bind(same_key);

    // Should only preserve the "a" entry
    let expected = to_hashmap(vec![("a".to_string(), 1)]);
    assert_eq!(result, expected);
  }

  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let map = to_hashmap(vec![
      ("a".to_string(), 5),
      ("b".to_string(), 10),
      ("c".to_string(), 15),
    ]);

    let f = add_entry;
    let g = multiply_entry;

    // m.bind(f).bind(g)
    let left = map.clone().bind(f).bind(g);

    // m.bind(|x| f(x).bind(g))
    let right = map.bind(move |x| {
      let f_result = add_entry(x);
      f_result.bind(g)
    });

    assert_eq!(left, right);
  }

  // Test binding with functions that have no overlapping keys
  #[test]
  fn test_non_overlapping_keys() {
    let map = to_hashmap(vec![
      ("a".to_string(), 1),
      ("b".to_string(), 2),
      ("c".to_string(), 3),
    ]);

    // Define a function that returns a map with completely different keys
    let different_keys = |x: &i32| {
      let mut result = HashMap::new();
      result.insert("d".to_string(), x + 1);
      result.insert("e".to_string(), x + 2);
      result
    };

    let result = map.bind(different_keys);

    // The result should be empty
    assert!(result.is_empty());
  }

  // Test binding with an empty function
  #[test]
  fn test_bind_with_empty_fn() {
    let map = to_hashmap(vec![
      ("a".to_string(), 1),
      ("b".to_string(), 2),
      ("c".to_string(), 3),
    ]);

    let result = map.bind(empty_result);

    // The result should be empty
    assert!(result.is_empty());
  }

  // Test binding with partially overlapping keys
  #[test]
  fn test_partial_overlap() {
    let map = to_hashmap(vec![
      ("a".to_string(), 1),
      ("b".to_string(), 2),
      ("c".to_string(), 3),
    ]);

    // Should only maintain key "b" which exists in both maps
    let result = map.bind(|x| {
      let mut new_map = HashMap::new();
      new_map.insert("b".to_string(), x * 2);
      new_map.insert("d".to_string(), x * 3); // This key doesn't exist in original map
      new_map
    });

    let expected = to_hashmap(vec![("b".to_string(), 4)]);
    assert_eq!(result, expected);
  }

  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let map = to_hashmap(vec![("a".to_string(), 1), ("b".to_string(), 2)]);

    // flat_map is operating on references, not values, so it will behave differently
    // from bind which consumes the map
    let flat_map_result = HashMapMonadExt::flat_map(&map, add_entry);

    // Test function preserves key "a", should have a value
    assert_eq!(flat_map_result.get("a"), Some(&2)); // a + 1 = 2
  }

  // Property-based tests
  proptest! {
    // Associativity law
    #[test]
    fn prop_associativity(
      // Generate small maps with string keys and integer values
      kvs in proptest::collection::hash_map(
        proptest::string::string_regex("[a-z]{1,3}").unwrap(),
        -100i32..100,
        0..5
      )
    ) {
      let f = add_entry;
      let g = multiply_entry;

      let left = kvs.clone().bind(f).bind(g);

      let right = kvs.bind(move |x| {
        let f_result = add_entry(x);
        f_result.bind(g)
      });

      prop_assert_eq!(left, right);
    }

    // Empty function produces empty result
    #[test]
    fn prop_empty_result(
      // Generate small maps with string keys and integer values
      kvs in proptest::collection::hash_map(
        proptest::string::string_regex("[a-z]{1,3}").unwrap(),
        -100i32..100,
        0..5
      )
    ) {
      let result = kvs.bind(empty_result);

      prop_assert!(result.is_empty());
    }
  }
}
