use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeMap;

impl<K, V> Semigroup for BTreeMap<K, V>
where
  K: Ord + CloneableThreadSafe,
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
  use proptest::prelude::*;
  use std::collections::BTreeMap;

  // Test basic combination of two maps with disjoint keys
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

  // Test combining empty maps
  #[test]
  fn test_combine_empty_maps() {
    let map1: BTreeMap<i32, String> = BTreeMap::new();
    let map2: BTreeMap<i32, String> = BTreeMap::new();

    let result = map1.combine(map2);
    assert!(result.is_empty());
  }

  // Test combining an empty map with a non-empty map
  #[test]
  fn test_combine_empty_with_nonempty() {
    let empty_map: BTreeMap<i32, String> = BTreeMap::new();

    let mut nonempty_map = BTreeMap::new();
    nonempty_map.insert(1, "one".to_string());
    nonempty_map.insert(2, "two".to_string());

    // Empty combined with non-empty should yield non-empty
    let result1 = empty_map.clone().combine(nonempty_map.clone());
    assert_eq!(result1, nonempty_map.clone());

    // Non-empty combined with empty should yield non-empty
    let result2 = nonempty_map.clone().combine(empty_map.clone());
    assert_eq!(result2, nonempty_map.clone());
  }

  // Test semigroup associativity with disjoint keys
  #[test]
  fn test_associativity_disjoint_keys() {
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "two".to_string());

    let mut map3 = BTreeMap::new();
    map3.insert(3, "three".to_string());

    // (map1 ⊕ map2) ⊕ map3
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());

    // map1 ⊕ (map2 ⊕ map3)
    let result2 = map1.clone().combine(map2.clone().combine(map3.clone()));

    assert_eq!(result1, result2);
  }

  // Test associativity with overlapping keys to show it's not always associative
  #[test]
  fn test_associativity_overlapping_keys() {
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one A".to_string());
    map1.insert(2, "two A".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "two B".to_string());
    map2.insert(3, "three B".to_string());

    let mut map3 = BTreeMap::new();
    map3.insert(3, "three C".to_string());
    map3.insert(4, "four C".to_string());

    // (map1 ⊕ map2) ⊕ map3
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());

    // map1 ⊕ (map2 ⊕ map3)
    let result2 = map1.clone().combine(map2.clone().combine(map3.clone()));

    // With overlapping keys, the results can be different due to right-biased behavior
    // Create the expected results to verify our understanding

    // (map1 ⊕ map2) has {1: "one A", 2: "two B", 3: "three B"}
    // Then combined with map3 gives {1: "one A", 2: "two B", 3: "three C", 4: "four C"}
    let mut expected1 = BTreeMap::new();
    expected1.insert(1, "one A".to_string());
    expected1.insert(2, "two B".to_string());
    expected1.insert(3, "three C".to_string());
    expected1.insert(4, "four C".to_string());

    // (map2 ⊕ map3) has {2: "two B", 3: "three C", 4: "four C"}
    // Then combined with map1 gives {1: "one A", 2: "two B", 3: "three C", 4: "four C"}
    let mut expected2 = BTreeMap::new();
    expected2.insert(1, "one A".to_string());
    expected2.insert(2, "two B".to_string());
    expected2.insert(3, "three C".to_string());
    expected2.insert(4, "four C".to_string());

    assert_eq!(result1, expected1);
    assert_eq!(result2, expected2);

    // In this specific example, the results happen to be the same because the right-most
    // values win in both cases. Let's create a more illustrative example:

    let mut map_a = BTreeMap::new();
    map_a.insert(1, "A1".to_string());

    let mut map_b = BTreeMap::new();
    map_b.insert(1, "B1".to_string());

    let mut map_c = BTreeMap::new();
    map_c.insert(1, "C1".to_string());

    // (A ⊕ B) ⊕ C = {1: "B1"} ⊕ C = {1: "C1"}
    let result_a = map_a.clone().combine(map_b.clone()).combine(map_c.clone());

    // A ⊕ (B ⊕ C) = A ⊕ {1: "C1"} = {1: "C1"}
    let result_b = map_a.clone().combine(map_b.clone().combine(map_c.clone()));

    // They happen to be the same again, because rightmost values always win
    assert_eq!(result_a, result_b);

    // Let's try one more case where merge order matters:
    let mut map_x = BTreeMap::new();
    map_x.insert(1, "X1".to_string());
    map_x.insert(2, "X2".to_string());

    let mut map_y = BTreeMap::new();
    map_y.insert(2, "Y2".to_string());

    let mut map_z = BTreeMap::new();
    map_z.insert(1, "Z1".to_string());

    // (X ⊕ Y) ⊕ Z = {1: "X1", 2: "Y2"} ⊕ Z = {1: "Z1", 2: "Y2"}
    let result_x = map_x.clone().combine(map_y.clone()).combine(map_z.clone());

    // X ⊕ (Y ⊕ Z) = X ⊕ {1: "Z1", 2: "Y2"} = {1: "Z1", 2: "Y2"}
    let result_y = map_x.clone().combine(map_y.clone().combine(map_z.clone()));

    // They are the same in this case too, due to how extend works in BTreeMap
    assert_eq!(result_x, result_y);
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

    let combined = str_map1.clone().combine(str_map2.clone());

    let mut expected = BTreeMap::new();
    expected.insert("apple".to_string(), 1);
    expected.insert("banana".to_string(), 2);
    expected.insert("cherry".to_string(), 3);
    expected.insert("date".to_string(), 4);

    assert_eq!(combined, expected);

    // Test commutativity with different types
    let combined2 = str_map2.clone().combine(str_map1.clone());

    let mut expected2 = BTreeMap::new();
    expected2.insert("apple".to_string(), 1);
    expected2.insert("banana".to_string(), 2);
    expected2.insert("cherry".to_string(), 3);
    expected2.insert("date".to_string(), 4);

    assert_eq!(combined2, expected2);
  }

  // Test all possible combine permutations of three maps
  #[test]
  fn test_combine_permutations() {
    // Create different maps with disjoint keys
    let mut map1 = BTreeMap::new();
    map1.insert(1, "one".to_string());

    let mut map2 = BTreeMap::new();
    map2.insert(2, "two".to_string());

    let mut map3 = BTreeMap::new();
    map3.insert(3, "three".to_string());

    // All six possible permutations
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());
    let result2 = map1.clone().combine(map3.clone()).combine(map2.clone());
    let result3 = map2.clone().combine(map1.clone()).combine(map3.clone());
    let result4 = map2.clone().combine(map3.clone()).combine(map1.clone());
    let result5 = map3.clone().combine(map1.clone()).combine(map2.clone());
    let result6 = map3.clone().combine(map2.clone()).combine(map1.clone());

    // Create the expected result - all maps should combine to the same result
    // with disjoint keys
    let mut expected = BTreeMap::new();
    expected.insert(1, "one".to_string());
    expected.insert(2, "two".to_string());
    expected.insert(3, "three".to_string());

    assert_eq!(result1, expected);
    assert_eq!(result2, expected);
    assert_eq!(result3, expected);
    assert_eq!(result4, expected);
    assert_eq!(result5, expected);
    assert_eq!(result6, expected);
  }

  // Test permutations with overlapping keys, showcasing that the last map's
  // values win for overlapping keys
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

    // Expected results for each permutation, based on the right-biased extend behavior

    // map1 -> map2 -> map3
    let result1 = map1.clone().combine(map2.clone()).combine(map3.clone());
    let mut expected1 = BTreeMap::new();
    expected1.insert(1, "one A".to_string());
    expected1.insert(2, "two B".to_string());
    expected1.insert(3, "three C".to_string());
    expected1.insert(4, "four C".to_string());
    assert_eq!(result1, expected1);

    // map1 -> map3 -> map2
    let result2 = map1.clone().combine(map3.clone()).combine(map2.clone());
    let mut expected2 = BTreeMap::new();
    expected2.insert(1, "one A".to_string());
    expected2.insert(2, "two B".to_string());
    expected2.insert(3, "three B".to_string());
    expected2.insert(4, "four C".to_string());
    assert_eq!(result2, expected2);

    // map2 -> map1 -> map3
    let result3 = map2.clone().combine(map1.clone()).combine(map3.clone());
    let mut expected3 = BTreeMap::new();
    expected3.insert(1, "one A".to_string());
    expected3.insert(2, "two A".to_string());
    expected3.insert(3, "three C".to_string());
    expected3.insert(4, "four C".to_string());
    assert_eq!(result3, expected3);

    // map2 -> map3 -> map1
    let result4 = map2.clone().combine(map3.clone()).combine(map1.clone());
    let mut expected4 = BTreeMap::new();
    expected4.insert(1, "one A".to_string());
    expected4.insert(2, "two A".to_string());
    expected4.insert(3, "three C".to_string());
    expected4.insert(4, "four C".to_string());
    assert_eq!(result4, expected4);

    // map3 -> map1 -> map2
    let result5 = map3.clone().combine(map1.clone()).combine(map2.clone());
    let mut expected5 = BTreeMap::new();
    expected5.insert(1, "one A".to_string());
    expected5.insert(2, "two B".to_string());
    expected5.insert(3, "three B".to_string());
    expected5.insert(4, "four C".to_string());
    assert_eq!(result5, expected5);

    // map3 -> map2 -> map1
    let result6 = map3.clone().combine(map2.clone()).combine(map1.clone());
    let mut expected6 = BTreeMap::new();
    expected6.insert(1, "one A".to_string());
    expected6.insert(2, "two A".to_string());
    expected6.insert(3, "three B".to_string());
    expected6.insert(4, "four C".to_string());
    assert_eq!(result6, expected6);

    // Show that with overlapping keys, the results are not all equal
    assert_ne!(result1, result3);
    assert_ne!(result1, result4);
    assert_ne!(result1, result6);
    assert_ne!(result2, result3);
    assert_ne!(result2, result4);
    assert_ne!(result2, result6);
    assert_ne!(result5, result3);
    assert_ne!(result5, result4);
    assert_ne!(result5, result6);
  }

  // Property-based tests
  proptest! {
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

    // Property test for commutativity with disjoint keys
    #[test]
    fn prop_commutativity_disjoint_keys(
      a_pairs in proptest::collection::vec((1..100i32, ".*".prop_map(String::from)), 0..5),
      b_pairs in proptest::collection::vec((100..200i32, ".*".prop_map(String::from)), 0..5)
    ) {
      let map_a: BTreeMap<i32, String> = a_pairs.into_iter().collect();
      let map_b: BTreeMap<i32, String> = b_pairs.into_iter().collect();

      let a_combine_b = map_a.clone().combine(map_b.clone());
      let b_combine_a = map_b.clone().combine(map_a.clone());

      prop_assert_eq!(a_combine_b, b_combine_a);
    }

    // Property test for non-commutativity with overlapping keys
    #[test]
    fn prop_noncommutativity_overlapping_keys(
      // Use same key range to ensure overlap
      a_pairs in proptest::collection::vec((1..50i32, "a_.*".prop_map(String::from)), 1..5),
      b_pairs in proptest::collection::vec((1..50i32, "b_.*".prop_map(String::from)), 1..5)
    ) {
      let map_a: BTreeMap<i32, String> = a_pairs.clone().into_iter().collect();
      let map_b: BTreeMap<i32, String> = b_pairs.clone().into_iter().collect();

      // Only test if there's actual overlap
      let a_keys: std::collections::HashSet<_> = map_a.keys().cloned().collect();
      let b_keys: std::collections::HashSet<_> = map_b.keys().cloned().collect();
      let overlapping_keys: Vec<_> = a_keys.intersection(&b_keys).collect();

      if !overlapping_keys.is_empty() {
        let a_combine_b = map_a.clone().combine(map_b.clone());
        let b_combine_a = map_b.clone().combine(map_a.clone());

        // Check that at least one overlapping key has different values
        let mut has_different_values = false;
        for &k in &overlapping_keys {
          // If the values for the same key are different in the original maps,
          // then the combined maps should also be different
          if map_a[k] != map_b[k] {
            has_different_values = true;
            // The value in a_combine_b should be from map_b for this key
            prop_assert_eq!(&a_combine_b[k], &map_b[k]);
            // The value in b_combine_a should be from map_a for this key
            prop_assert_eq!(&b_combine_a[k], &map_a[k]);
          }
        }

        if has_different_values {
          prop_assert_ne!(a_combine_b, b_combine_a);
        }
      }
    }

    // Property test for empty map behavior
    #[test]
    fn prop_empty_map_identity(
      pairs in proptest::collection::vec((any::<i32>(), ".*".prop_map(String::from)), 0..10)
    ) {
      let map: BTreeMap<i32, String> = pairs.into_iter().collect();
      let empty_map: BTreeMap<i32, String> = BTreeMap::new();

      // empty ⊕ map = map
      let left = empty_map.clone().combine(map.clone());
      prop_assert_eq!(left, map.clone());

      // map ⊕ empty = map
      let right = map.clone().combine(empty_map.clone());
      prop_assert_eq!(right, map.clone());
    }
  }
}
