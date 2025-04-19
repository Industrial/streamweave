use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::HashSet;
use std::hash::Hash;

impl<T> Monoid for HashSet<T>
where
  T: Eq + Hash + CloneableThreadSafe,
{
  fn empty() -> Self {
    HashSet::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  // Basic tests for the empty operation
  #[test]
  fn test_empty() {
    let empty_set: HashSet<i32> = HashSet::empty();
    assert!(empty_set.is_empty());
  }

  // Test that empty set is the left identity
  #[test]
  fn test_left_identity() {
    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let empty_set = HashSet::empty();
    let result = empty_set.combine(set.clone());
    assert_eq!(result, set);
  }

  // Test that empty set is the right identity
  #[test]
  fn test_right_identity() {
    let mut set = HashSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let empty_set = HashSet::empty();
    let result = set.clone().combine(empty_set);
    assert_eq!(result, set);
  }

  // Test that combining with an empty set is identity
  #[test]
  fn test_identity_with_empty() {
    let empty_set: HashSet<i32> = HashSet::empty();
    let result = empty_set.clone().combine(empty_set.clone());
    assert_eq!(result, HashSet::empty());
  }

  // Test associativity with multiple HashSets
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

  // Test that mconcat works correctly
  #[test]
  fn test_mconcat() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);

    let mut set3 = HashSet::new();
    set3.insert(5);
    set3.insert(6);

    let result = HashSet::mconcat(vec![set1, set2, set3]);

    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);
    expected.insert(6);

    assert_eq!(result, expected);
  }

  // Test with different types
  #[test]
  fn test_with_different_types() {
    // Test with strings
    let mut set1 = HashSet::new();
    set1.insert("hello".to_string());
    set1.insert("world".to_string());

    let mut set2 = HashSet::new();
    set2.insert("foo".to_string());
    set2.insert("bar".to_string());

    let result = set1.clone().combine(set2.clone());

    let mut expected = HashSet::new();
    expected.insert("hello".to_string());
    expected.insert("world".to_string());
    expected.insert("foo".to_string());
    expected.insert("bar".to_string());

    assert_eq!(result, expected);
  }

  // Test all combine permutations
  #[test]
  fn test_combine_permutations() {
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
    let result1 = set1.clone().combine(set2.clone().combine(set3.clone()));
    let result2 = set1.clone().combine(set3.clone().combine(set2.clone()));
    let result3 = set2.clone().combine(set1.clone().combine(set3.clone()));
    let result4 = set2.clone().combine(set3.clone().combine(set1.clone()));
    let result5 = set3.clone().combine(set1.clone().combine(set2.clone()));
    let result6 = set3.clone().combine(set2.clone().combine(set1.clone()));

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

  // Test combining sets with overlapping elements
  #[test]
  fn test_combine_overlapping_elements() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);
    set2.insert(5);

    let result = set1.clone().combine(set2.clone());

    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);

    assert_eq!(result, expected);
  }

  // Test combining disjoint sets
  #[test]
  fn test_combine_disjoint_elements() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);

    let mut set2 = HashSet::new();
    set2.insert(4);
    set2.insert(5);
    set2.insert(6);

    let result = set1.clone().combine(set2.clone());

    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);
    expected.insert(6);

    assert_eq!(result, expected);
  }

  // Test with tuple type
  #[test]
  fn test_with_tuple_type() {
    let mut set1 = HashSet::new();
    set1.insert((1, "one".to_string()));
    set1.insert((2, "two".to_string()));

    let mut set2 = HashSet::new();
    set2.insert((3, "three".to_string()));
    set2.insert((4, "four".to_string()));

    let result = set1.clone().combine(set2.clone());

    let mut expected = HashSet::new();
    expected.insert((1, "one".to_string()));
    expected.insert((2, "two".to_string()));
    expected.insert((3, "three".to_string()));
    expected.insert((4, "four".to_string()));

    assert_eq!(result, expected);
  }

  // Test mconcat with empty iterator
  #[test]
  fn test_mconcat_all_empty() {
    let empty_vec: Vec<HashSet<i32>> = vec![];
    let result = HashSet::mconcat(empty_vec);
    assert_eq!(result, HashSet::empty());
  }

  // Test mconcat with a mix of empty and non-empty sets
  #[test]
  fn test_mconcat_mixed_empty() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);

    let empty_set: HashSet<i32> = HashSet::empty();

    let mut set3 = HashSet::new();
    set3.insert(3);
    set3.insert(4);

    let result = HashSet::mconcat(vec![set1, empty_set, set3]);

    let mut expected = HashSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);

    assert_eq!(result, expected);
  }

  // Test that mconcat is equivalent to folding with combine
  #[test]
  fn test_mconcat_equivalence() {
    let mut set1 = HashSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = HashSet::new();
    set2.insert(3);
    set2.insert(4);

    let mut set3 = HashSet::new();
    set3.insert(5);
    set3.insert(6);

    let sets = vec![set1.clone(), set2.clone(), set3.clone()];

    let mconcat_result = HashSet::mconcat(sets.clone());
    let fold_result = sets
      .into_iter()
      .fold(HashSet::empty(), |acc, x| acc.combine(x));

    assert_eq!(mconcat_result, fold_result);
  }

  // Property-based tests

  // Test that empty is a left identity
  proptest! {
    #[test]
    fn prop_left_identity(elements in prop::collection::hash_set(1..100i32, 0..10)) {
      let empty_set = HashSet::<i32>::empty();
      let result = empty_set.combine(elements.clone());
      prop_assert_eq!(result, elements);
    }
  }

  // Test that empty is a right identity
  proptest! {
    #[test]
    fn prop_right_identity(elements in prop::collection::hash_set(1..100i32, 0..10)) {
      let empty_set = HashSet::<i32>::empty();
      let result = elements.clone().combine(empty_set);
      prop_assert_eq!(result, elements);
    }
  }

  // Test associativity
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

  // Test that the empty set is consistent
  proptest! {
    #[test]
    fn prop_empty_consistent(_dummy in 0..10) {
      let empty1 = HashSet::<i32>::empty();
      let empty2 = HashSet::<i32>::empty();
      prop_assert_eq!(empty1, empty2);
    }
  }

  // Test commutativity (this is a specific property of HashSet, not required by Monoid)
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

  // Test that mconcat is equivalent to fold
  proptest! {
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      xss in prop::collection::vec(prop::collection::hash_set(1..100i32, 0..5), 0..10)
    ) {
      let mconcat_result = HashSet::mconcat(xss.clone());
      let fold_result = xss.into_iter().fold(HashSet::empty(), |acc, x| acc.combine(x));
      prop_assert_eq!(mconcat_result, fold_result);
    }
  }
}
