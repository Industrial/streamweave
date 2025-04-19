use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashSet;
use std::hash::Hash;

impl<T> Semigroup for HashSet<T>
where
  T: Eq + Hash + CloneableThreadSafe,
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

  // Test basic combine functionality
  #[test]
  fn test_combine_basic() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);

    let result = set1.combine(set2);

    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);

    assert_eq!(result, expected);
  }

  // Test combining with empty sets
  #[test]
  fn test_combine_empties() {
    let empty1: HashSet<i32> = HashSet::new();
    let empty2: HashSet<i32> = HashSet::new();

    let result = empty1.combine(empty2);
    assert!(result.is_empty());
  }

  // Test combining empty with non-empty
  #[test]
  fn test_combine_with_empty() {
    let empty: HashSet<i32> = HashSet::new();

    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);

    // Empty + Non-empty
    let result1 = empty.clone().combine(set.clone());
    assert_eq!(result1, set);

    // Non-empty + Empty
    let result2 = set.clone().combine(empty);
    assert_eq!(result2, set);
  }

  // Test combining overlapping sets
  #[test]
  fn test_combine_overlapping() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);
    set2.insert(5);

    let result = set1.combine(set2);

    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);

    assert_eq!(result, expected);
  }

  // Test associativity property
  #[test]
  fn test_associativity() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);

    let mut set3 = HashSet::new();
    set3.insert(5);
    set3.insert(6);

    // (set1 combine set2) combine set3
    let result1 = set1.clone().combine(set2.clone()).combine(set3.clone());

    // set1 combine (set2 combine set3)
    let result2 = set1.clone().combine(set2.clone().combine(set3.clone()));

    assert_eq!(result1, result2);
  }

  // Test all combination permutations
  #[test]
  fn test_all_combine_permutations() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);

    let mut set3 = HashSet::new();
    set3.insert(5);
    set3.insert(6);

    // All possible ways to combine three sets
    let result1 = set1.clone().combine(set2.clone()).combine(set3.clone());
    let result2 = set1.clone().combine(set3.clone().combine(set2.clone()));
    let result3 = set2.clone().combine(set1.clone()).combine(set3.clone());
    let result4 = set2.clone().combine(set3.clone()).combine(set1.clone());
    let result5 = set3.clone().combine(set1.clone()).combine(set2.clone());
    let result6 = set3.clone().combine(set2.clone()).combine(set1.clone());

    // All permutations should produce the same set
    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);
    expected.insert(6);

    assert_eq!(result1, expected);
    assert_eq!(result2, expected);
    assert_eq!(result3, expected);
    assert_eq!(result4, expected);
    assert_eq!(result5, expected);
    assert_eq!(result6, expected);
  }

  // Test with string type
  #[test]
  fn test_with_string_type() {
    let mut set1 = HashSet::new();
    set1.insert("hello".to_string());
    set1.insert("world".to_string());

    let mut set2 = HashSet::new();
    set2.insert("foo".to_string());
    set2.insert("bar".to_string());

    let result = set1.combine(set2);

    let mut expected = HashSet::new();
    expected.insert("hello".to_string());
    expected.insert("world".to_string());
    expected.insert("foo".to_string());
    expected.insert("bar".to_string());

    assert_eq!(result, expected);
  }

  // Property-based tests

  // Test associativity property
  proptest! {
    #[test]
    fn prop_associativity(
      xs in prop::collection::hash_set(1..100i32, 0..5),
      ys in prop::collection::hash_set(1..100i32, 0..5),
      zs in prop::collection::hash_set(1..100i32, 0..5)
    ) {
      let result1 = xs.clone().combine(ys.clone()).combine(zs.clone());
      let result2 = xs.clone().combine(ys.clone().combine(zs.clone()));
      prop_assert_eq!(result1, result2);
    }
  }

  // Test commutativity property (a specific property of HashSet, not required by Semigroup)
  proptest! {
    #[test]
    fn prop_commutativity(
      xs in prop::collection::hash_set(1..100i32, 0..5),
      ys in prop::collection::hash_set(1..100i32, 0..5)
    ) {
      let result1 = xs.clone().combine(ys.clone());
      let result2 = ys.clone().combine(xs.clone());
      prop_assert_eq!(result1, result2);
    }
  }

  // Test that combine preserves all elements
  proptest! {
    #[test]
    fn prop_combine_preserves_elements(
      xs in prop::collection::hash_set(1..100i32, 0..5),
      ys in prop::collection::hash_set(1..100i32, 0..5)
    ) {
      let combined = xs.clone().combine(ys.clone());

      // Check that all elements from both sets are in the result
      for x in xs.iter() {
        prop_assert!(combined.contains(x));
      }

      for y in ys.iter() {
        prop_assert!(combined.contains(y));
      }

      // Check that the result doesn't have any extra elements
      for e in combined.iter() {
        prop_assert!(xs.contains(e) || ys.contains(e));
      }
    }
  }

  // Test that the size of the combined set is correct
  proptest! {
    #[test]
    fn prop_combine_size(
      xs in prop::collection::hash_set(1..100i32, 0..5),
      ys in prop::collection::hash_set(1..100i32, 0..5)
    ) {
      let combined = xs.clone().combine(ys.clone());

      // Calculate expected size - the union of two sets
      // We need to count elements that are in xs, plus elements in ys that are not in xs
      let mut expected_size = xs.len();
      for y in ys.iter() {
        if !xs.contains(y) {
          expected_size += 1;
        }
      }

      prop_assert_eq!(combined.len(), expected_size);
    }
  }
}
