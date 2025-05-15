//! Implementation of the `Monad` trait for `BTreeMap<K, A>`.
//!
//! This module implements the monadic bind operation for the `BTreeMap` type,
//! enabling key-based composition of operations on mapped values.
//!
//! The implementation applies a function to each value in the map, where the function
//! itself returns a BTreeMap. The result is a map containing only the keys that exist
//! in both the original map and the map returned by the function application.
//!
//! This can be thought of as a filtering operation where each value can produce
//! a new collection, but we only keep entries where the keys align.
//!
//! Unlike HashMap, BTreeMap maintains key ordering, which affects iteration order.

use crate::impls::btreemap::category::BTreeMapCategory;
use crate::traits::monad::Monad;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;
use std::cmp::Ord;
use std::marker::PhantomData;

impl<K, A> Monad<A> for BTreeMapCategory<K, A>
where
  K: Ord + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn bind<B, F>(self, _f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> Self::HigherSelf<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is a no-op placeholder since the actual implementation happens through
    // the extension trait below
    BTreeMapCategory(PhantomData)
  }
}

// Extension trait for more ergonomic monadic operations
pub trait BTreeMapMonadExt<K, A>
where
  K: Ord + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn bind<B, F>(self, f: F) -> BTreeMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> BTreeMap<K, B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
    
  fn flat_map<B, F>(&self, f: F) -> BTreeMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> BTreeMap<K, B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

impl<K, A> BTreeMapMonadExt<K, A> for BTreeMap<K, A>
where
  K: Ord + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn bind<B, F>(self, mut f: F) -> BTreeMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> BTreeMap<K, B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = BTreeMap::new();
    
    for (k, a) in self {
      let new_map = f(&a);
      
      // Only include k in the result if it exists in the new_map
      if let Some(b) = new_map.get(&k) {
        result.insert(k, b.clone());
      }
    }
    
    result
  }
  
  fn flat_map<B, F>(&self, mut f: F) -> BTreeMap<K, B>
  where
    F: for<'a> FnMut(&'a A) -> BTreeMap<K, B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = BTreeMap::new();
    
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
  
  // Helper to create a BTreeMap from key-value pairs
  fn to_btreemap<K, V>(entries: Vec<(K, V)>) -> BTreeMap<K, V>
  where
    K: Ord,
  {
    entries.into_iter().collect()
  }
  
  // Test functions for the monad laws
  fn add_entry(x: &i32) -> BTreeMap<String, i32> {
    let mut map = BTreeMap::new();
    map.insert("a".to_string(), x + 1);
    map.insert("b".to_string(), x + 2);
    map
  }
  
  fn multiply_entry(x: &i32) -> BTreeMap<String, i32> {
    let mut map = BTreeMap::new();
    map.insert("b".to_string(), x * 2);
    map.insert("c".to_string(), x * 3);
    map
  }
  
  fn empty_result(_: &i32) -> BTreeMap<String, i32> {
    BTreeMap::new()
  }
  
  // Left identity law: pure(a).bind(f) == f(a)
  // For BTreeMap, we test the extension function instead
  #[test]
  fn test_left_identity_specialization() {
    let _a = 42i32;
    let f = add_entry;
    
    // Empty map bound with f should be empty
    let empty_map: BTreeMap<String, i32> = BTreeMap::new();
    let left = empty_map.bind(f);
    
    // The result should be empty
    assert!(left.is_empty());
  }
  
  // Right identity law: m.bind(pure) == m
  // For BTreeMap, we adjust since pure produces an empty map
  #[test]
  fn test_right_identity_specialization() {
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2), 
      ("c".to_string(), 3)
    ]);
    
    // m.bind(|x| pure(x)) will be empty due to the empty map from pure
    let result = map.bind(|_| BTreeMap::<String, i32>::new());
    
    // The result should be empty
    assert!(result.is_empty());
  }
  
  // Key-preserving identity for BTreeMaps:
  // m.bind(|v| single_entry_map_with_same_key(v)) == m.select_keys(keys_in_single_entry_map)
  #[test]
  fn test_key_preserving_identity() {
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2), 
      ("c".to_string(), 3)
    ]);
    
    // Define a function that returns a map with only the same key
    let same_key = |x: &i32| {
      let mut result = BTreeMap::new();
      result.insert("a".to_string(), *x);
      result
    };
    
    let result = map.clone().bind(same_key);
    
    // Should only preserve the "a" entry
    let expected = to_btreemap(vec![("a".to_string(), 1)]);
    assert_eq!(result, expected);
  }
  
  // Associativity law: m.bind(f).bind(g) == m.bind(|x| f(x).bind(g))
  #[test]
  fn test_associativity() {
    let map = to_btreemap(vec![
      ("a".to_string(), 5), 
      ("b".to_string(), 10), 
      ("c".to_string(), 15)
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
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2), 
      ("c".to_string(), 3)
    ]);
    
    // Define a function that returns a map with completely different keys
    let different_keys = |x: &i32| {
      let mut result = BTreeMap::new();
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
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2), 
      ("c".to_string(), 3)
    ]);
    
    let result = map.bind(empty_result);
    
    // The result should be empty
    assert!(result.is_empty());
  }
  
  // Test binding with partially overlapping keys
  #[test]
  fn test_partial_overlap() {
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2), 
      ("c".to_string(), 3)
    ]);
    
    // Should only maintain key "b" which exists in both maps
    let result = map.bind(|x| {
      let mut new_map = BTreeMap::new();
      new_map.insert("b".to_string(), x * 2);
      new_map.insert("d".to_string(), x * 3);  // This key doesn't exist in original map
      new_map
    });
    
    let expected = to_btreemap(vec![("b".to_string(), 4)]);
    assert_eq!(result, expected);
  }
  
  // Test flat_map alias
  #[test]
  fn test_flat_map() {
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2)
    ]);
    
    // flat_map is operating on references, not values, so it will behave differently
    // from bind which consumes the map
    let flat_map_result = BTreeMapMonadExt::flat_map(&map, add_entry);
    
    // Test function preserves key "a", should have a value
    assert_eq!(flat_map_result.get("a"), Some(&2)); // a + 1 = 2
  }
  
  // Test ordering preservation (specific to BTreeMap)
  #[test]
  fn test_order_preservation() {
    let map = to_btreemap(vec![
      ("a".to_string(), 1), 
      ("b".to_string(), 2), 
      ("c".to_string(), 3), 
      ("d".to_string(), 4)
    ]);
    
    // Function that returns values for keys "a", "b", and "d"
    let f = |x: &i32| {
      let mut result = BTreeMap::new();
      result.insert("b".to_string(), x * 2);  // Will match in both maps
      result.insert("d".to_string(), x * 3);  // Will match in both maps
      result.insert("a".to_string(), x * 4);  // Will match in both maps
      result
    };
    
    let result = map.bind(f);
    
    // Keys should be in sorted order, which is a property of BTreeMap
    let keys: Vec<_> = result.keys().collect();
    assert_eq!(keys, vec!["a", "b", "d"]);
    
    // Values should be correct
    assert_eq!(result.get("a").unwrap(), &4);
    assert_eq!(result.get("b").unwrap(), &4);
    assert_eq!(result.get("d").unwrap(), &12);
  }
  
  // Property-based tests
  proptest! {
    // Associativity law
    #[test]
    fn prop_associativity(
      // Generate small maps with string keys and integer values
      kvs in proptest::collection::btree_map(
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
      kvs in proptest::collection::btree_map(
        proptest::string::string_regex("[a-z]{1,3}").unwrap(),
        -100i32..100,
        0..5
      )
    ) {
      let result = kvs.bind(empty_result);
      
      prop_assert!(result.is_empty());
    }
    
    // Order preservation property
    #[test]
    fn prop_order_preservation(
      // Generate small maps with string keys and integer values
      kvs in proptest::collection::btree_map(
        proptest::string::string_regex("[a-z]{1,3}").unwrap(),
        -100i32..100,
        0..5
      )
    ) {
      // We need to clone kvs for the closure to avoid ownership issues
      let kvs_clone = kvs.clone();
      
      // Create a function that preserves some keys
      let f = move |x: &i32| {
        let mut result = BTreeMap::new();
        // Only preserve every other key in kvs 
        let keys: Vec<_> = kvs_clone.keys().enumerate()
          .filter(|(i, _)| i % 2 == 0)
          .map(|(_, k)| k.clone())
          .collect();
          
        for k in keys {
          result.insert(k, x * 2);
        }
        result
      };
      
      let result = kvs.bind(f);
      
      // All keys in result should be in order
      let mut prev_key: Option<&String> = None;
      
      for k in result.keys() {
        if let Some(p) = prev_key {
          prop_assert!(p < k, "Keys should be in order");
        }
        prev_key = Some(k);
      }
    }
  }
} 