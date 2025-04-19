use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BTreeSet;

impl<T> Monoid for BTreeSet<T>
where
  T: Ord + CloneableThreadSafe,
{
  fn empty() -> Self {
    BTreeSet::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;
  use std::collections::BTreeSet;

  // Test that empty() returns an empty set
  #[test]
  fn test_empty() {
    let empty_set: BTreeSet<i32> = Monoid::empty();
    assert!(empty_set.is_empty());
    assert_eq!(empty_set.len(), 0);
  }

  // Test left identity: empty().combine(x) = x
  #[test]
  fn test_left_identity() {
    let mut set = BTreeSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let empty_set: BTreeSet<i32> = Monoid::empty();
    let result = empty_set.combine(set.clone());

    assert_eq!(result, set);
  }

  // Test right identity: x.combine(empty()) = x
  #[test]
  fn test_right_identity() {
    let mut set = BTreeSet::new();
    set.insert(1);
    set.insert(2);
    set.insert(3);

    let empty_set: BTreeSet<i32> = Monoid::empty();
    let result = set.clone().combine(empty_set);

    assert_eq!(result, set);
  }

  // Test identity with an empty set
  #[test]
  fn test_identity_with_empty() {
    let set: BTreeSet<i32> = BTreeSet::new();
    let empty_set: BTreeSet<i32> = Monoid::empty();

    let left_result = empty_set.clone().combine(set.clone());
    let right_result = set.clone().combine(empty_set);

    assert!(left_result.is_empty());
    assert!(right_result.is_empty());
  }

  // Test combining sets with disjoint elements
  #[test]
  fn test_combine_disjoint_elements() {
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
  fn test_combine_overlapping_elements() {
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

    // For sets, combine should be commutative (unlike maps)
    let result2 = set2.clone().combine(set1.clone());
    assert_eq!(result, result2);
  }

  // Test all permutations of combining three sets
  #[test]
  fn test_combine_permutations() {
    // Create different sets
    let mut set1 = BTreeSet::new();
    set1.insert(1);
    set1.insert(2);

    let mut set2 = BTreeSet::new();
    set2.insert(3);
    set2.insert(4);

    let mut set3 = BTreeSet::new();
    set3.insert(5);

    // Expected result - all sets should combine to the same result
    let mut expected = BTreeSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);
    expected.insert(5);

    // All six possible permutations
    let result1 = set1.clone().combine(set2.clone()).combine(set3.clone());
    let result2 = set1.clone().combine(set3.clone()).combine(set2.clone());
    let result3 = set2.clone().combine(set1.clone()).combine(set3.clone());
    let result4 = set2.clone().combine(set3.clone()).combine(set1.clone());
    let result5 = set3.clone().combine(set1.clone()).combine(set2.clone());
    let result6 = set3.clone().combine(set2.clone()).combine(set1.clone());

    assert_eq!(result1, expected);
    assert_eq!(result2, expected);
    assert_eq!(result3, expected);
    assert_eq!(result4, expected);
    assert_eq!(result5, expected);
    assert_eq!(result6, expected);
  }

  // Test all permutations of combining four sets
  #[test]
  fn test_combine_permutations_four_sets() {
    // Create four different sets
    let mut set1 = BTreeSet::new();
    set1.insert(1);

    let mut set2 = BTreeSet::new();
    set2.insert(2);

    let mut set3 = BTreeSet::new();
    set3.insert(3);

    let mut set4 = BTreeSet::new();
    set4.insert(4);

    // Expected result - all sets should combine to the same result
    let mut expected = BTreeSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);
    expected.insert(4);

    // Test some key permutations (testing all 24 would be excessive)
    let result1 = set1
      .clone()
      .combine(set2.clone())
      .combine(set3.clone())
      .combine(set4.clone());
    let result2 = set1
      .clone()
      .combine(set3.clone())
      .combine(set2.clone())
      .combine(set4.clone());
    let result3 = set4
      .clone()
      .combine(set3.clone())
      .combine(set2.clone())
      .combine(set1.clone());

    // Also test different groupings with parentheses
    let result4 = set1
      .clone()
      .combine(set2.clone().combine(set3.clone()).combine(set4.clone()));
    let result5 = set1
      .clone()
      .combine(set2.clone().combine(set3.clone()))
      .combine(set4.clone());

    assert_eq!(result1, expected);
    assert_eq!(result2, expected);
    assert_eq!(result3, expected);
    assert_eq!(result4, expected);
    assert_eq!(result5, expected);
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

  // Test with different types
  #[test]
  fn test_with_different_types() {
    // String elements
    let mut str_set1 = BTreeSet::new();
    str_set1.insert("apple".to_string());
    str_set1.insert("banana".to_string());

    let mut str_set2 = BTreeSet::new();
    str_set2.insert("cherry".to_string());
    str_set2.insert("date".to_string());

    let empty_str_set: BTreeSet<String> = Monoid::empty();

    // Test identity laws with strings
    assert_eq!(
      empty_str_set.clone().combine(str_set1.clone()),
      str_set1.clone()
    );

    assert_eq!(
      str_set1.clone().combine(empty_str_set.clone()),
      str_set1.clone()
    );

    // Test combining with strings
    let combined = str_set1.clone().combine(str_set2.clone());

    let mut expected = BTreeSet::new();
    expected.insert("apple".to_string());
    expected.insert("banana".to_string());
    expected.insert("cherry".to_string());
    expected.insert("date".to_string());

    assert_eq!(combined, expected);

    // Test commutativity with different types
    let combined2 = str_set2.clone().combine(str_set1.clone());
    assert_eq!(combined, combined2);
  }

  // Test with a more complex type
  #[test]
  fn test_with_tuple_type() {
    let mut tuple_set1 = BTreeSet::new();
    tuple_set1.insert((1, "one".to_string()));
    tuple_set1.insert((2, "two".to_string()));

    let mut tuple_set2 = BTreeSet::new();
    tuple_set2.insert((3, "three".to_string()));
    tuple_set2.insert((1, "one".to_string())); // Duplicated key to test deduplication

    let combined = tuple_set1.clone().combine(tuple_set2.clone());

    let mut expected = BTreeSet::new();
    expected.insert((1, "one".to_string()));
    expected.insert((2, "two".to_string()));
    expected.insert((3, "three".to_string()));

    assert_eq!(combined, expected);
    assert_eq!(combined.len(), 3); // Should be 3 since (1, "one") appears in both sets
  }

  // Test mconcat with multiple sets
  #[test]
  fn test_mconcat() {
    let mut set1 = BTreeSet::new();
    set1.insert(1);

    let mut set2 = BTreeSet::new();
    set2.insert(2);

    let mut set3 = BTreeSet::new();
    set3.insert(3);

    let sets = vec![set1, set2, set3];
    let result = BTreeSet::mconcat(sets);

    let mut expected = BTreeSet::new();
    expected.insert(1);
    expected.insert(2);
    expected.insert(3);

    assert_eq!(result, expected);

    // Test with empty list
    let empty_sets: Vec<BTreeSet<i32>> = vec![];
    let empty_result = BTreeSet::mconcat(empty_sets);
    assert!(empty_result.is_empty());
  }

  // Test mconcat with all empty sets
  #[test]
  fn test_mconcat_all_empty() {
    let empty1: BTreeSet<i32> = BTreeSet::empty();
    let empty2: BTreeSet<i32> = BTreeSet::empty();
    let empty3: BTreeSet<i32> = BTreeSet::empty();

    let sets = vec![empty1, empty2, empty3];
    let result = BTreeSet::mconcat(sets);

    assert!(result.is_empty());
    assert_eq!(result, BTreeSet::empty());
  }

  // Test mconcat with mixed empty and non-empty sets
  #[test]
  fn test_mconcat_mixed_empty() {
    let empty1: BTreeSet<i32> = BTreeSet::empty();

    let mut set2 = BTreeSet::new();
    set2.insert(2);

    let empty3: BTreeSet<i32> = BTreeSet::empty();

    let mut set4 = BTreeSet::new();
    set4.insert(4);

    let sets = vec![empty1, set2.clone(), empty3, set4.clone()];
    let result = BTreeSet::mconcat(sets);

    let mut expected = BTreeSet::new();
    expected.insert(2);
    expected.insert(4);

    assert_eq!(result, expected);
  }

  // Test that mconcat is equivalent to folding with combine
  #[test]
  fn test_mconcat_equivalence() {
    let mut set1 = BTreeSet::new();
    set1.insert(1);

    let mut set2 = BTreeSet::new();
    set2.insert(2);
    set2.insert(3);

    let mut set3 = BTreeSet::new();
    set3.insert(3);
    set3.insert(4);

    let sets = vec![set1.clone(), set2.clone(), set3.clone()];

    let mconcat_result = BTreeSet::mconcat(sets.clone());
    let fold_result = sets
      .into_iter()
      .fold(BTreeSet::empty(), |acc, x| acc.combine(x));

    assert_eq!(mconcat_result, fold_result);
  }

  // Property-based tests for monoid laws
  proptest! {
    // Property-based test for left identity
    #[test]
    fn prop_left_identity(elements in proptest::collection::vec(any::<i32>(), 0..10)) {
      let set: BTreeSet<i32> = elements.iter().cloned().collect();
      let empty_set: BTreeSet<i32> = Monoid::empty();

      prop_assert_eq!(empty_set.combine(set.clone()), set);
    }

    // Property-based test for right identity
    #[test]
    fn prop_right_identity(elements in proptest::collection::vec(any::<i32>(), 0..10)) {
      let set: BTreeSet<i32> = elements.iter().cloned().collect();
      let empty_set: BTreeSet<i32> = Monoid::empty();

      prop_assert_eq!(set.clone().combine(empty_set), set);
    }

    // Property test for associativity
    #[test]
    fn prop_associativity(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5),
      c in proptest::collection::vec(any::<i32>(), 0..5)
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

    // Property test for commutativity
    #[test]
    fn prop_commutativity(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5)
    ) {
      let set_a: BTreeSet<i32> = a.into_iter().collect();
      let set_b: BTreeSet<i32> = b.into_iter().collect();

      let a_combine_b = set_a.clone().combine(set_b.clone());
      let b_combine_a = set_b.clone().combine(set_a.clone());

      prop_assert_eq!(a_combine_b, b_combine_a);
    }

    // Property-based test for mconcat equivalence to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      sets in proptest::collection::vec(
        proptest::collection::vec(any::<i32>(), 0..5).prop_map(|v| {
          let set: BTreeSet<i32> = v.into_iter().collect();
          set
        }),
        0..5
      )
    ) {
      let mconcat_result = BTreeSet::mconcat(sets.clone());

      let fold_result = sets.into_iter().fold(
        BTreeSet::empty(),
        |acc, x| acc.combine(x)
      );

      prop_assert_eq!(mconcat_result, fold_result);
    }

    // Property test for combining preserves all elements
    #[test]
    fn prop_combine_preserves_elements(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5)
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

    // Property test that empty() always produces the same result
    #[test]
    fn prop_empty_consistent(dummy in any::<i32>()) {
      // We ignore the dummy value, it's just to drive the proptest engine
      let _ = dummy;

      let empty1: BTreeSet<i32> = Monoid::empty();
      let empty2: BTreeSet<i32> = Monoid::empty();

      prop_assert!(empty1.is_empty());
      prop_assert!(empty2.is_empty());
      prop_assert_eq!(empty1.len(), empty2.len());
    }
  }
}
