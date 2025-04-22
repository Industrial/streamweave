use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;
use std::hash::Hash;

/// Implementation of Applicative for HashMap.
///
/// For HashMaps, we interpret:
/// - `pure(x)` as an empty HashMap, since we can't create a map with a value without a key
/// - `ap` as applying functions from one HashMap to values in another HashMap where keys match
impl<K, A> Applicative<A> for HashMap<K, A>
where
  K: Eq + Hash + Clone + CloneableThreadSafe,
  A: CloneableThreadSafe,
{
  fn pure<B>(_value: B) -> Self::HigherSelf<B>
  where
    B: CloneableThreadSafe,
  {
    // For HashMaps, pure produces an empty map since we can't create a map with a value without a key
    HashMap::new()
  }

  fn ap<B, F>(self, fs: Self::HigherSelf<F>) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = HashMap::new();

    // For each key that exists in both self and fs, apply the function to the value
    for (k, v) in self {
      if let Some(mut f) = fs.get(&k).cloned() {
        let new_value = f(&v);
        result.insert(k, new_value);
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

  // Simple function types for our tests
  type IdentityFn = fn(&i32) -> i32;
  type StringFn = fn(&i32) -> String;

  // Helper functions for tests
  fn double(x: &i32) -> i32 {
    x * 2
  }
  fn add_ten(x: &i32) -> i32 {
    x + 10
  }
  fn square(x: &i32) -> i32 {
    x * x
  }
  fn to_string(x: &i32) -> String {
    x.to_string()
  }
  fn format_number(x: &i32) -> String {
    format!("Number: {}", x)
  }
  fn format_value(x: &i32) -> String {
    format!("{} is the value", x)
  }

  // Identity law: pure(id) <*> v = v
  // Note: For HashMap, since pure produces an empty map, this test verifies
  // that applying an empty map of functions results in an empty map
  #[test]
  fn test_empty_map_ap() {
    let map = to_hashmap(vec![("a", 1), ("b", 2), ("c", 3)]);

    // Empty map of functions
    let empty_fs: HashMap<&str, IdentityFn> = HashMap::new();

    // An empty map of functions applied to any map should result in an empty map
    let result = map.ap(empty_fs);
    assert!(result.is_empty());
  }

  // Test map with matching keys
  #[test]
  fn test_ap_with_matching_keys() {
    let map = to_hashmap(vec![("a", 1), ("b", 2), ("c", 3)]);

    // Create functions to apply to each value
    let mut functions: HashMap<&str, IdentityFn> = HashMap::new();
    functions.insert("a", double);
    functions.insert("b", add_ten);
    functions.insert("c", square);

    let result = map.ap(functions);

    let expected = to_hashmap(vec![
      ("a", 2),  // 1 * 2
      ("b", 12), // 2 + 10
      ("c", 9),  // 3 * 3
    ]);

    assert_eq!(result, expected);
  }

  // Test with partial key overlap
  #[test]
  fn test_ap_with_partial_overlap() {
    let map = to_hashmap(vec![("a", 1), ("b", 2), ("c", 3)]);

    let mut functions: HashMap<&str, IdentityFn> = HashMap::new();
    functions.insert("a", double);
    functions.insert("c", add_ten);
    functions.insert("d", square); // no match in map

    let result = map.ap(functions);

    let expected = to_hashmap(vec![
      ("a", 2), // 1 * 2
      ("c", 13), // 3 + 10
                // "b" and "d" are not in the result
    ]);

    assert_eq!(result, expected);
  }

  // Type conversion test - Using explicit type annotation for the HashMap
  #[test]
  fn test_ap_with_type_conversion() {
    let map = to_hashmap(vec![("a", 1), ("b", 2), ("c", 3)]);

    // Use explicit type annotation to avoid closure type mismatches
    let mut functions: HashMap<&str, StringFn> = HashMap::new();
    functions.insert("a", to_string);
    functions.insert("b", format_number);
    functions.insert("c", format_value);

    let result = map.ap(functions);

    let expected = to_hashmap(vec![
      ("a", "1".to_string()),
      ("b", "Number: 2".to_string()),
      ("c", "3 is the value".to_string()),
    ]);

    assert_eq!(result, expected);
  }

  // For property tests, we'll use explicit function pointers
  fn add_one(x: &i32) -> i32 {
    x + 1
  }

  // Function enum for proptest
  #[derive(Debug, Clone, Copy)]
  enum TestFunc {
    AddOne,
    Double,
    Square,
  }

  proptest! {
      #[test]
      fn prop_ap_preserves_input_keys(
          // Generate small maps with string keys and integer values
          kvs in proptest::collection::hash_map(
              proptest::string::string_regex("[a-z]{1,5}").unwrap(),
              0i32..10,
              1..5
          ),
          // Generate similar maps for function identifiers
          fns in proptest::collection::hash_map(
              proptest::string::string_regex("[a-z]{1,5}").unwrap(),
              prop::sample::select(vec![
                  TestFunc::AddOne,
                  TestFunc::Double,
                  TestFunc::Square,
              ]),
              1..5
          )
      ) {
          // Convert function identifiers to actual function pointers
          let mut functions: HashMap<String, IdentityFn> = HashMap::new();
          for (k, func_id) in fns {
              let func = match func_id {
                  TestFunc::AddOne => add_one,
                  TestFunc::Double => double,
                  TestFunc::Square => square,
              };
              functions.insert(k.clone(), func);
          }

          let result = kvs.clone().ap(functions.clone());

          // The result should only contain keys that appear in both maps
          for k in result.keys() {
              prop_assert!(kvs.contains_key(k));
              prop_assert!(functions.contains_key(k));
          }

          // Each value should be correctly transformed
          for (k, v) in &result {
              let original = kvs.get(k).unwrap();
              let f = functions.get(k).unwrap();
              prop_assert_eq!(*v, f(original));
          }
      }
  }
}
