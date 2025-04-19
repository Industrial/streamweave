use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;

impl<K, V> Monoid for BTreeMap<K, V>
where
  K: Ord + CloneableThreadSafe,
  V: CloneableThreadSafe,
{
  fn empty() -> Self {
    BTreeMap::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;
  use std::collections::BTreeMap;

  // Test that empty() returns an empty map
  #[test]
  fn test_empty() {
    let empty_map: BTreeMap<i32, String> = Monoid::empty();
    assert!(empty_map.is_empty());
    assert_eq!(empty_map.len(), 0);
  }

  // Test left identity: empty().combine(x) = x
  #[test]
  fn test_left_identity() {
    let mut map = BTreeMap::new();
    map.insert(1, "one".to_string());
    map.insert(2, "two".to_string());
    map.insert(3, "three".to_string());

    let empty_map: BTreeMap<i32, String> = Monoid::empty();
    let result = empty_map.combine(map.clone());

    assert_eq!(result, map);
  }

  // Test right identity: x.combine(empty()) = x
  #[test]
  fn test_right_identity() {
    let mut map = BTreeMap::new();
    map.insert(1, "one".to_string());
    map.insert(2, "two".to_string());
    map.insert(3, "three".to_string());

    let empty_map: BTreeMap<i32, String> = Monoid::empty();
    let result = map.clone().combine(empty_map);

    assert_eq!(result, map);
  }

  // Test identity with an empty map
  #[test]
  fn test_identity_with_empty() {
    let map: BTreeMap<i32, String> = BTreeMap::new();
    let empty_map: BTreeMap<i32, String> = Monoid::empty();

    let left_result = empty_map.clone().combine(map.clone());
    let right_result = map.clone().combine(empty_map);

    assert!(left_result.is_empty());
    assert!(right_result.is_empty());
  }

  // Test combining maps with disjoint keys
  #[test]
  fn test_combine_disjoint_keys() {
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one".to_string());
    map1.insert(2, "two".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(3, "three".to_string());
    map2.insert(4, "four".to_string());

    let result = map1.clone().combine(map2.clone());

    let mut expected = BTreeMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "two".to_string());
    expected.insert(3, "three".to_string());
    expected.insert(4, "four".to_string());

    assert_eq!(result, expected);
  }

  // Test combining maps with overlapping keys
  #[test]
  fn test_combine_overlapping_keys() {
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one".to_string());
    map1.insert(2, "two".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "TWO".to_string());
    map2.insert(3, "three".to_string());

    let result = map1.clone().combine(map2.clone());

    // We expect the right-hand map's values to win for overlapping keys
    let mut expected = BTreeMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "TWO".to_string());
    expected.insert(3, "three".to_string());

    assert_eq!(result, expected);
  }

  // Test all permutations of combining maps
  #[test]
  fn test_combine_permutations() {
    // Create different maps
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one".to_string());
    map1.insert(3, "three".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "two".to_string());
    map2.insert(4, "four".to_string());

    let mut map3 = BTreeMap::new();
    map3.insert(5, "five".to_string());

    // Expected results when each map has disjoint keys
    let mut expected = BTreeMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "two".to_string());
    expected.insert(3, "three".to_string());
    expected.insert(4, "four".to_string());
    expected.insert(5, "five".to_string());

    // Different permutations of combining the maps
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());
    let result2 = map1.clone().combine(map3.clone()).combine(map2.clone());
    let result3 = map2.clone().combine(map1.clone()).combine(map3.clone());
    let result4 = map2.clone().combine(map3.clone()).combine(map1.clone());
    let result5 = map3.clone().combine(map1.clone()).combine(map2.clone());
    let result6 = map3.clone().combine(map2.clone()).combine(map1.clone());

    // With disjoint keys, all permutations should result in the same combined map
    assert_eq!(result1, expected);
    assert_eq!(result2, expected);
    assert_eq!(result3, expected);
    assert_eq!(result4, expected);
    assert_eq!(result5, expected);
    assert_eq!(result6, expected);
  }

  // Test permutations with overlapping keys
  #[test]
  fn test_combine_permutations_with_overlapping_keys() {
    // Create maps with overlapping keys
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one A".to_string());
    map1.insert(2, "two A".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "two B".to_string());
    map2.insert(3, "three B".to_string());

    let mut map3 = BTreeMap::new();
    map3.insert(3, "three C".to_string());
    map3.insert(4, "four C".to_string());

    // Different permutations will result in different outcomes due to key overlap

    // (map1 ⊕ map2) ⊕ map3
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());
    let mut expected1 = BTreeMap::new();
    expected1.insert(1, "one A".to_string());
    expected1.insert(2, "two B".to_string());
    expected1.insert(3, "three C".to_string());
    expected1.insert(4, "four C".to_string());
    assert_eq!(result1, expected1);

    // (map1 ⊕ map3) ⊕ map2
    let result2 = map1.clone().combine(map3.clone()).combine(map2.clone());
    let mut expected2 = BTreeMap::new();
    expected2.insert(1, "one A".to_string());
    expected2.insert(2, "two B".to_string());
    expected2.insert(3, "three B".to_string());
    expected2.insert(4, "four C".to_string());
    assert_eq!(result2, expected2);

    // (map2 ⊕ map3) ⊕ map1
    let result3 = map2.clone().combine(map3.clone()).combine(map1.clone());
    let mut expected3 = BTreeMap::new();
    expected3.insert(1, "one A".to_string());
    expected3.insert(2, "two A".to_string());
    expected3.insert(3, "three C".to_string());
    expected3.insert(4, "four C".to_string());
    assert_eq!(result3, expected3);

    // These assertions show that BTreeMap monoid is not associative when keys overlap
    assert_ne!(result1, result2);
    assert_ne!(result1, result3);
    assert_ne!(result2, result3);
  }

  // Test with different types
  #[test]
  fn test_with_different_types() {
    // String keys, integer values
    let mut str_map1 = BTreeMap::new();
    str_map1.insert("apple".to_string(), 1);
    str_map1.insert("banana".to_string(), 2);

    let mut str_map2 = BTreeMap::new();
    str_map2.insert("cherry".to_string(), 3);
    str_map2.insert("date".to_string(), 4);

    let empty_str_map: BTreeMap<String, i32> = Monoid::empty();

    // Test identity laws with string keys
    assert_eq!(
      empty_str_map.clone().combine(str_map1.clone()),
      str_map1.clone()
    );

    assert_eq!(
      str_map1.clone().combine(empty_str_map.clone()),
      str_map1.clone()
    );

    // Test combining with string keys
    let combined = str_map1.clone().combine(str_map2.clone());

    let mut expected = BTreeMap::new();
    expected.insert("apple".to_string(), 1);
    expected.insert("banana".to_string(), 2);
    expected.insert("cherry".to_string(), 3);
    expected.insert("date".to_string(), 4);

    assert_eq!(combined, expected);
  }

  // Test associativity law with disjoint keys
  #[test]
  fn test_associativity_disjoint_keys() {
    // Create test maps with disjoint keys
    let mut map_a = BTreeMap::new();
    map_a.insert(1, "one".to_string());

    let mut map_b = BTreeMap::new();
    map_b.insert(2, "two".to_string());

    let mut map_c = BTreeMap::new();
    map_c.insert(3, "three".to_string());

    // (a ⊕ b) ⊕ c
    let left = map_a.clone().combine(map_b.clone()).combine(map_c.clone());

    // a ⊕ (b ⊕ c)
    let right = map_a.clone().combine(map_b.clone().combine(map_c.clone()));

    assert_eq!(left, right);
  }

  // Test mconcat with multiple maps
  #[test]
  fn test_mconcat() {
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "two".to_string());

    let mut map3 = BTreeMap::new();
    map3.insert(3, "three".to_string());

    let maps = vec![map1, map2, map3];
    let result = BTreeMap::mconcat(maps);

    let mut expected = BTreeMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "two".to_string());
    expected.insert(3, "three".to_string());

    assert_eq!(result, expected);

    // Test with empty list
    let empty_maps: Vec<BTreeMap<i32, String>> = vec![];
    let empty_result = BTreeMap::mconcat(empty_maps);
    assert!(empty_result.is_empty());
  }

  // Property-based tests for monoid laws
  proptest! {
    // Property-based test for left identity
    #[test]
    fn prop_left_identity(
      pairs in proptest::collection::vec((any::<i32>(), ".*".prop_map(String::from)), 0..10)
    ) {
      let map: BTreeMap<i32, String> = pairs.into_iter().collect();
      let empty_map: BTreeMap<i32, String> = Monoid::empty();

      prop_assert_eq!(empty_map.combine(map.clone()), map);
    }

    // Property-based test for right identity
    #[test]
    fn prop_right_identity(
      pairs in proptest::collection::vec((any::<i32>(), ".*".prop_map(String::from)), 0..10)
    ) {
      let map: BTreeMap<i32, String> = pairs.into_iter().collect();
      let empty_map: BTreeMap<i32, String> = Monoid::empty();

      prop_assert_eq!(map.clone().combine(empty_map), map);
    }

    // Property-based test that ensures empty() always produces
    // the same result regardless of the type parameter
    #[test]
    fn prop_empty_consistent(dummy in any::<i32>()) {
      // We ignore the dummy value, it's just to drive the proptest engine
      let _ = dummy;

      let empty1: BTreeMap<i32, String> = Monoid::empty();
      let empty2: BTreeMap<i32, String> = Monoid::empty();

      prop_assert!(empty1.is_empty());
      prop_assert!(empty2.is_empty());
      prop_assert_eq!(empty1.len(), empty2.len());
    }

    // Property test for associativity with disjoint keys
    #[test]
    fn prop_associativity_disjoint_keys(
      a_pairs in proptest::collection::vec((1..100i32, ".*".prop_map(String::from)), 0..5),
      b_pairs in proptest::collection::vec((100..200i32, ".*".prop_map(String::from)), 0..5),
      c_pairs in proptest::collection::vec((200..300i32, ".*".prop_map(String::from)), 0..5)
    ) {
      let map_a: BTreeMap<i32, String> = a_pairs.into_iter().collect();
      let map_b: BTreeMap<i32, String> = b_pairs.into_iter().collect();
      let map_c: BTreeMap<i32, String> = c_pairs.into_iter().collect();

      // (a ⊕ b) ⊕ c
      let left = map_a.clone().combine(map_b.clone()).combine(map_c.clone());

      // a ⊕ (b ⊕ c)
      let right = map_a.clone().combine(map_b.clone().combine(map_c.clone()));

      prop_assert_eq!(left, right);
    }

    // Property test for right-biased behavior with overlapping keys
    #[test]
    fn prop_right_biased_combine(
      base_pairs in proptest::collection::vec((any::<i32>(), "base_.*".prop_map(String::from)), 0..10),
      override_pairs in proptest::collection::vec((any::<i32>(), "override_.*".prop_map(String::from)), 0..10)
    ) {
      let base_map: BTreeMap<i32, String> = base_pairs.into_iter().collect();
      let override_map: BTreeMap<i32, String> = override_pairs.into_iter().collect();

      let combined = base_map.clone().combine(override_map.clone());

      // For each key in combined map
      for (k, v) in combined.iter() {
        // If the key is in override_map, its value should win
        if override_map.contains_key(k) {
          prop_assert_eq!(v, &override_map[k]);
        } else {
          // Otherwise the value should come from base_map
          prop_assert_eq!(v, &base_map[k]);
        }
      }

      // The combined map should have all keys from both maps
      for k in base_map.keys() {
        prop_assert!(combined.contains_key(k));
      }

      for k in override_map.keys() {
        prop_assert!(combined.contains_key(k));
      }
    }

    // Property test for mconcat equivalence to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      maps in proptest::collection::vec(
        proptest::collection::vec((any::<i32>(), ".*".prop_map(String::from)), 0..5)
          .prop_map(|pairs| pairs.into_iter().collect::<BTreeMap<i32, String>>()),
        0..5
      )
    ) {
      let mconcat_result = BTreeMap::mconcat(maps.clone());

      let fold_result = maps.into_iter().fold(
        BTreeMap::empty(),
        |acc, x| acc.combine(x)
      );

      prop_assert_eq!(mconcat_result, fold_result);
    }

    // Property test for key preservation in combine
    #[test]
    fn prop_combine_preserves_keys(
      a_pairs in proptest::collection::vec((any::<i32>(), ".*".prop_map(String::from)), 0..5),
      b_pairs in proptest::collection::vec((any::<i32>(), ".*".prop_map(String::from)), 0..5)
    ) {
      let map_a: BTreeMap<i32, String> = a_pairs.into_iter().collect();
      let map_b: BTreeMap<i32, String> = b_pairs.into_iter().collect();

      let combined = map_a.clone().combine(map_b.clone());

      // Check that all keys from both maps are in the combined map
      let mut all_keys = Vec::new();
      for k in map_a.keys() {
        all_keys.push(*k);
        prop_assert!(combined.contains_key(k));
      }

      for k in map_b.keys() {
        all_keys.push(*k);
        prop_assert!(combined.contains_key(k));
      }

      // The combined map should not have any extra keys
      prop_assert_eq!(combined.len(), all_keys.into_iter().collect::<std::collections::HashSet<_>>().len());
    }
  }
}
