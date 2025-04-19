use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeSet;

impl<T> Semigroup for BTreeSet<T>
where
  T: Ord + CloneableThreadSafe,
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
    let mut set1 = BTreeSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = BTreeSet::new();
    set2.insert(3);
    set2.insert(4);

    let result = set1.clone().combine(set2.clone());

    let mut expected = BTreeSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);

    assert_eq!(result, expected);
  }

  // Test combining sets with overlapping elements
  #[test]
  fn test_combine_overlapping() {
    let mut set1 = BTreeSet::new();
    set1.insert(1);
    set1.insert(2);
    set1.insert(3);

    let mut set2 = BTreeSet::new();
    set2.insert(2);
    set2.insert(3);
    set2.insert(4);

    let result = set1.clone().combine(set2.clone());

    let mut expected = BTreeSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);

    assert_eq!(result, expected);
  }

  // Test associativity: (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
  #[test]
  fn test_associativity() {
    let mut set_a = BTreeSet::new();
    set_a.insert(1);
    set_a.insert(2);

    let mut set_b = BTreeSet::new();
    set_b.insert(2);
    set_b.insert(3);

    let mut set_c = BTreeSet::new();
    set_c.insert(3);
    set_c.insert(4);

    // (a ⊕ b) ⊕ c
    let left = set_a.clone().combine(set_b.clone()).combine(set_c.clone());

    // a ⊕ (b ⊕ c)
    let right = set_a.clone().combine(set_b.clone().combine(set_c.clone()));

    assert_eq!(left, right);
  }

  // Test combine with empty set
  #[test]
  fn test_combine_with_empty() {
    let mut set = BTreeSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let empty_set: BTreeSet<i32> = BTreeSet::new();

    // non-empty ⊕ empty
    let result1 = set.clone().combine(empty_set.clone());
    assert_eq!(result1, set);

    // empty ⊕ non-empty
    let result2 = empty_set.clone().combine(set.clone());
    assert_eq!(result2, set);
  }

  // Test empty ⊕ empty = empty
  #[test]
  fn test_combine_empties() {
    let empty_set1: BTreeSet<i32> = BTreeSet::new();
    let empty_set2: BTreeSet<i32> = BTreeSet::new();

    let result = empty_set1.combine(empty_set2);
    assert!(result.is_empty());
  }

  // Test all permutations of combining multiple sets
  #[test]
  fn test_all_combine_permutations() {
    // Create three different sets
    let mut set1 = BTreeSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = BTreeSet::new();
    set2.insert(3);
    set2.insert(4);

    let mut set3 = BTreeSet::new();
    set3.insert(5);
    set3.insert(6);

    // Expected result - all sets should combine to the same result
    let mut expected = BTreeSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);
    expected.insert(6);

    // Test all 6 permutations of combining these three sets
    // 1-2-3
    let result1 = set1.clone().combine(set2.clone()).combine(set3.clone());
    assert_eq!(result1, expected);

    // 1-3-2
    let result2 = set1.clone().combine(set3.clone()).combine(set2.clone());
    assert_eq!(result2, expected);

    // 2-1-3
    let result3 = set2.clone().combine(set1.clone()).combine(set3.clone());
    assert_eq!(result3, expected);

    // 2-3-1
    let result4 = set2.clone().combine(set3.clone()).combine(set1.clone());
    assert_eq!(result4, expected);

    // 3-1-2
    let result5 = set3.clone().combine(set1.clone()).combine(set2.clone());
    assert_eq!(result5, expected);

    // 3-2-1
    let result6 = set3.clone().combine(set2.clone()).combine(set1.clone());
    assert_eq!(result6, expected);
  }

  // Test with different types
  #[test]
  fn test_with_string_type() {
    let mut string_set1 = BTreeSet::new();
    string_set1.insert("apple".to_string());
    string_set1.insert("banana".to_string());

    let mut string_set2 = BTreeSet::new();
    string_set2.insert("cherry".to_string());
    string_set2.insert("apple".to_string()); // Overlap with set1

    let result = string_set1.clone().combine(string_set2.clone());

    let mut expected = BTreeSet::new();
    expected.insert("apple".to_string());
    expected.insert("banana".to_string());
    expected.insert("cherry".to_string());

    assert_eq!(result, expected);
  }

  // Property-based tests for semigroup laws
  proptest! {
    // Property test for associativity
    #[test]
    fn prop_associativity(
      a in proptest::collection::vec(any::<i32>(), 0..10),
      b in proptest::collection::vec(any::<i32>(), 0..10),
      c in proptest::collection::vec(any::<i32>(), 0..10)
    ) {
      let set_a: BTreeSet<i32> = a.into_iter().collect();
      let set_b: BTreeSet<i32> = b.into_iter().collect();
      let set_c: BTreeSet<i32> = c.into_iter().collect();

      // (a ⊕ b) ⊕ c
      let left = set_a.clone().combine(set_b.clone()).combine(set_c.clone());

      // a ⊕ (b ⊕ c)
      let right = set_a.clone().combine(set_b.clone().combine(set_c.clone()));

      prop_assert_eq!(left, right);
    }

    // Property test for commutativity (sets have this additional property)
    #[test]
    fn prop_commutativity(
      a in proptest::collection::vec(any::<i32>(), 0..10),
      b in proptest::collection::vec(any::<i32>(), 0..10)
    ) {
      let set_a: BTreeSet<i32> = a.into_iter().collect();
      let set_b: BTreeSet<i32> = b.into_iter().collect();

      let a_combine_b = set_a.clone().combine(set_b.clone());
      let b_combine_a = set_b.clone().combine(set_a.clone());

      prop_assert_eq!(a_combine_b, b_combine_a);
    }

    // Property test for combine preserves all elements
    #[test]
    fn prop_combine_preserves_elements(
      a in proptest::collection::vec(any::<i32>(), 0..10),
      b in proptest::collection::vec(any::<i32>(), 0..10)
    ) {
      let set_a: BTreeSet<i32> = a.iter().cloned().collect();
      let set_b: BTreeSet<i32> = b.iter().cloned().collect();

      let combined = set_a.clone().combine(set_b.clone());

      // Check that all elements from both sets are in the combined set
      for &x in &a {
        prop_assert!(combined.contains(&x));
      }

      for &x in &b {
        prop_assert!(combined.contains(&x));
      }

      // The combined set should not have any extra elements
      for &x in &combined {
        prop_assert!(set_a.contains(&x) || set_b.contains(&x));
      }
    }
  }
}
