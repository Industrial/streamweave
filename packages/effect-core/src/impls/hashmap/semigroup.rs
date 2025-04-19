use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashMap;

impl<K, V> Semigroup for HashMap<K, V>
where
  K: Eq + std::hash::Hash + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn combine(mut self, other: Self) -> Self {
    self.extend(other);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::collection::hash_map;
  use proptest::prelude::*;
  use std::collections::HashMap;

  #[test]
  fn test_combine_empty_maps() {
    let map1: HashMap<i32, String> = HashMap::new();
    let map2: HashMap<i32, String> = HashMap::new();

    let result = map1.combine(map2);
    assert_eq!(result, HashMap::new());
  }

  #[test]
  fn test_combine_with_empty() {
    let mut map1 = HashMap::new();
    map1.insert(1, "one".to_string());
    map1.insert(2, "two".to_string());

    let map2: HashMap<i32, String> = HashMap::new();

    // Verify the clone for later assertions
    let map1_clone = map1.clone();

    // Empty map on right side doesn't change the result
    assert_eq!(map1.combine(map2), map1_clone);

    // Empty map on left side returns the right map
    let mut map3 = HashMap::new();
    map3.insert(3, "three".to_string());

    let empty_map: HashMap<i32, String> = HashMap::new();
    assert_eq!(empty_map.combine(map3.clone()), map3);
  }

  #[test]
  fn test_combine_disjoint_maps() {
    let mut map1 = HashMap::new();
    map1.insert(1, "one".to_string());
    map1.insert(2, "two".to_string());

    let mut map2 = HashMap::new();
    map2.insert(3, "three".to_string());
    map2.insert(4, "four".to_string());

    let mut expected = HashMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "two".to_string());
    expected.insert(3, "three".to_string());
    expected.insert(4, "four".to_string());

    assert_eq!(map1.combine(map2), expected);
  }

  #[test]
  fn test_combine_overlapping_maps() {
    let mut map1 = HashMap::new();
    map1.insert(1, "one".to_string());
    map1.insert(2, "two".to_string());

    let mut map2 = HashMap::new();
    map2.insert(2, "TWO".to_string()); // Key overlap with map1
    map2.insert(3, "three".to_string());

    let mut expected = HashMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "TWO".to_string()); // Second map's value wins
    expected.insert(3, "three".to_string());

    assert_eq!(map1.combine(map2), expected);
  }

  #[test]
  fn test_associativity_manual() {
    // Test with some predefined maps
    let mut map1 = HashMap::new();
    map1.insert(1, "one".to_string());

    let mut map2 = HashMap::new();
    map2.insert(2, "two".to_string());

    let mut map3 = HashMap::new();
    map3.insert(3, "three".to_string());

    test_associativity(map1, map2, map3);
  }

  // Property-based tests

  // Generate small HashMap<i32, String> instances for testing associativity
  proptest_semigroup_associativity!(
    prop_associativity_i32_string,
    HashMap<i32, String>,
    hash_map(
      any::<i32>().prop_filter("Value too large", |v| *v < 100),
      any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
      0..5
    )
  );

  proptest! {
    #[test]
    fn prop_combine_preserves_keys(
      map1 in hash_map(any::<i32>().prop_filter("Value too large", |v| *v < 100),
                     any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
                     0..5),
      map2 in hash_map(any::<i32>().prop_filter("Value too large", |v| *v < 100),
                     any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
                     0..5)
    ) {
      let combined = map1.clone().combine(map2.clone());

      // All keys from both maps should be present in the combined map
      for key in map1.keys() {
        prop_assert!(combined.contains_key(key));
      }

      for key in map2.keys() {
        prop_assert!(combined.contains_key(key));
      }
    }

    #[test]
    fn prop_right_map_values_win_on_conflict(
      // Generate maps with potential key conflicts
      map1 in hash_map(1..5i32, any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()), 1..3),
      map2 in hash_map(3..7i32, any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()), 1..3)
    ) {
      let combined = map1.clone().combine(map2.clone());

      // For overlapping keys, the second map's value should win
      for (key, value) in &map2 {
        if let Some(combined_value) = combined.get(key) {
          prop_assert_eq!(combined_value, value);
        }
      }
    }

    #[test]
    fn prop_combine_size(
      map1 in hash_map(any::<i32>().prop_filter("Value too large", |v| *v < 100),
                     any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
                     0..5),
      map2 in hash_map(any::<i32>().prop_filter("Value too large", |v| *v < 100),
                     any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
                     0..5)
    ) {
      let combined = map1.clone().combine(map2.clone());

      // Number of keys in combined map should be at most the sum of the original maps
      // (might be less if there are overlapping keys)
      prop_assert!(combined.len() <= map1.len() + map2.len());

      // Count overlapping keys
      let overlap_count = map2.keys().filter(|k| map1.contains_key(k)).count();

      // Combined map size should be exactly the sum minus overlaps
      prop_assert_eq!(combined.len(), map1.len() + map2.len() - overlap_count);
    }
  }
}
