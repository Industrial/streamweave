use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BinaryHeap;

impl<T> Monoid for BinaryHeap<T>
where
  T: Ord + CloneableThreadSafe,
{
  fn empty() -> Self {
    BinaryHeap::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  // Test that empty() returns an empty heap
  #[test]
  fn test_empty() {
    let empty_heap: BinaryHeap<i32> = Monoid::empty();
    assert!(empty_heap.is_empty());
    assert_eq!(empty_heap.len(), 0);
  }

  // Test left identity: empty().combine(x) = x
  #[test]
  fn test_left_identity() {
    let mut heap = BinaryHeap::new();
    heap.push(1);
    heap.push(2);
    heap.push(3);

    let empty_heap: BinaryHeap<i32> = Monoid::empty();
    let result = empty_heap.combine(heap.clone());

    assert_eq!(result.into_sorted_vec(), heap.into_sorted_vec());
  }

  // Test right identity: x.combine(empty()) = x
  #[test]
  fn test_right_identity() {
    let mut heap = BinaryHeap::new();
    heap.push(1);
    heap.push(2);
    heap.push(3);

    let empty_heap: BinaryHeap<i32> = Monoid::empty();
    let result = heap.clone().combine(empty_heap);

    assert_eq!(result.into_sorted_vec(), heap.into_sorted_vec());
  }

  // Test identity with an empty heap
  #[test]
  fn test_identity_with_empty() {
    let heap: BinaryHeap<i32> = BinaryHeap::new();
    let empty_heap: BinaryHeap<i32> = Monoid::empty();

    let left_result = empty_heap.clone().combine(heap.clone());
    let right_result = heap.clone().combine(empty_heap);

    assert!(left_result.is_empty());
    assert!(right_result.is_empty());
  }

  // Test all permutations of combining heaps with different elements
  #[test]
  fn test_combine_permutations() {
    // Create different heaps
    let mut heap1 = BinaryHeap::new();
    heap1.push(1);
    heap1.push(3);

    let mut heap2 = BinaryHeap::new();
    heap2.push(2);
    heap2.push(4);

    let mut heap3 = BinaryHeap::new();
    heap3.push(5);

    // Expected combined result
    let mut expected = BinaryHeap::new();
    expected.push(1);
    expected.push(2);
    expected.push(3);
    expected.push(4);
    expected.push(5);

    // Different permutations of combining the heaps
    let result1 = heap1.clone().combine(heap2.clone()).combine(heap3.clone());
    let result2 = heap1.clone().combine(heap3.clone()).combine(heap2.clone());
    let result3 = heap2.clone().combine(heap1.clone()).combine(heap3.clone());
    let result4 = heap2.clone().combine(heap3.clone()).combine(heap1.clone());
    let result5 = heap3.clone().combine(heap1.clone()).combine(heap2.clone());
    let result6 = heap3.clone().combine(heap2.clone()).combine(heap1.clone());

    // All permutations should result in the same combined heap
    assert_eq!(
      result1.into_sorted_vec(),
      expected.clone().into_sorted_vec()
    );
    assert_eq!(
      result2.into_sorted_vec(),
      expected.clone().into_sorted_vec()
    );
    assert_eq!(
      result3.into_sorted_vec(),
      expected.clone().into_sorted_vec()
    );
    assert_eq!(
      result4.into_sorted_vec(),
      expected.clone().into_sorted_vec()
    );
    assert_eq!(
      result5.into_sorted_vec(),
      expected.clone().into_sorted_vec()
    );
    assert_eq!(result6.into_sorted_vec(), expected.into_sorted_vec());
  }

  // Test associativity law: (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
  #[test]
  fn test_associativity() {
    // Create test heaps
    let mut heap_a = BinaryHeap::new();
    heap_a.push(1);
    heap_a.push(3);

    let mut heap_b = BinaryHeap::new();
    heap_b.push(2);

    let mut heap_c = BinaryHeap::new();
    heap_c.push(4);
    heap_c.push(5);

    // (a ⊕ b) ⊕ c
    let left = heap_a
      .clone()
      .combine(heap_b.clone())
      .combine(heap_c.clone());
    let left_sorted = left.into_sorted_vec();

    // a ⊕ (b ⊕ c)
    let right = heap_a
      .clone()
      .combine(heap_b.clone().combine(heap_c.clone()));
    let right_sorted = right.into_sorted_vec();

    assert_eq!(left_sorted, right_sorted);
  }

  // Test with different types
  #[test]
  fn test_with_different_types() {
    // Test with strings
    let mut str_heap1 = BinaryHeap::new();
    str_heap1.push("apple");
    str_heap1.push("banana");

    let mut str_heap2 = BinaryHeap::new();
    str_heap2.push("cherry");
    str_heap2.push("date");

    let empty_str_heap: BinaryHeap<&str> = Monoid::empty();

    // Test identity laws with strings
    assert_eq!(
      empty_str_heap
        .clone()
        .combine(str_heap1.clone())
        .into_sorted_vec(),
      str_heap1.clone().into_sorted_vec()
    );

    assert_eq!(
      str_heap1
        .clone()
        .combine(empty_str_heap.clone())
        .into_sorted_vec(),
      str_heap1.clone().into_sorted_vec()
    );

    // Test associativity with strings
    let combined1 = str_heap1.clone().combine(str_heap2.clone());
    let combined2 = str_heap2.clone().combine(str_heap1.clone());

    // Order matters for the heap itself but not for the sorted results
    assert_eq!(combined1.into_sorted_vec(), combined2.into_sorted_vec());
  }

  // Test mconcat with multiple heaps
  #[test]
  fn test_mconcat() {
    let mut heap1 = BinaryHeap::new();
    heap1.push(1);

    let mut heap2 = BinaryHeap::new();
    heap2.push(2);

    let mut heap3 = BinaryHeap::new();
    heap3.push(3);

    let heaps = vec![heap1, heap2, heap3];
    let result = BinaryHeap::mconcat(heaps);

    let mut expected = BinaryHeap::new();
    expected.push(1);
    expected.push(2);
    expected.push(3);

    assert_eq!(result.into_sorted_vec(), expected.into_sorted_vec());

    // Test with empty list
    let empty_heaps: Vec<BinaryHeap<i32>> = vec![];
    let empty_result = BinaryHeap::mconcat(empty_heaps);
    assert!(empty_result.is_empty());
  }

  // Property-based tests for monoid laws
  proptest! {
    // Property-based test for left identity
    #[test]
    fn prop_left_identity(elements in proptest::collection::vec(any::<i32>(), 0..10)) {
      let heap: BinaryHeap<i32> = elements.iter().cloned().collect();
      let empty_heap: BinaryHeap<i32> = Monoid::empty();

      let result = empty_heap.combine(heap.clone());

      prop_assert_eq!(result.into_sorted_vec(), heap.into_sorted_vec());
    }

    // Property-based test for right identity
    #[test]
    fn prop_right_identity(elements in proptest::collection::vec(any::<i32>(), 0..10)) {
      let heap: BinaryHeap<i32> = elements.iter().cloned().collect();
      let empty_heap: BinaryHeap<i32> = Monoid::empty();

      let result = heap.clone().combine(empty_heap);

      prop_assert_eq!(result.into_sorted_vec(), heap.into_sorted_vec());
    }

    // Property-based test combining the identity law
    // with associativity (a * e) * b = a * (e * b)
    #[test]
    fn prop_identity_associativity(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5)
    ) {
      let heap_a: BinaryHeap<i32> = a.iter().cloned().collect();
      let heap_b: BinaryHeap<i32> = b.iter().cloned().collect();
      let empty_heap: BinaryHeap<i32> = Monoid::empty();

      // (a * e) * b
      let left = heap_a.clone().combine(empty_heap.clone()).combine(heap_b.clone());
      let left_sorted = left.clone().into_sorted_vec();

      // a * (e * b)
      let right = heap_a.clone().combine(empty_heap.combine(heap_b.clone()));

      // Direct combination without empty
      let direct = heap_a.combine(heap_b);

      prop_assert_eq!(left_sorted, right.into_sorted_vec());
      prop_assert_eq!(left.into_sorted_vec(), direct.into_sorted_vec());
    }

    // Property-based test that ensures empty() always produces
    // the same result regardless of the type parameter
    #[test]
    fn prop_empty_consistent(dummy in any::<i32>()) {
      // We ignore the dummy value, it's just to drive the proptest engine
      let _ = dummy;

      let empty1: BinaryHeap<i32> = Monoid::empty();
      let empty2: BinaryHeap<i32> = Monoid::empty();

      prop_assert!(empty1.is_empty());
      prop_assert!(empty2.is_empty());
      prop_assert_eq!(empty1.len(), empty2.len());
    }

    // Property test for associativity with three heaps
    #[test]
    fn prop_associativity(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5),
      c in proptest::collection::vec(any::<i32>(), 0..5)
    ) {
      let heap_a: BinaryHeap<i32> = a.iter().cloned().collect();
      let heap_b: BinaryHeap<i32> = b.iter().cloned().collect();
      let heap_c: BinaryHeap<i32> = c.iter().cloned().collect();

      // (a ⊕ b) ⊕ c
      let left = heap_a.clone().combine(heap_b.clone()).combine(heap_c.clone());

      // a ⊕ (b ⊕ c)
      let right = heap_a.clone().combine(heap_b.clone().combine(heap_c.clone()));

      prop_assert_eq!(left.into_sorted_vec(), right.into_sorted_vec());
    }

    // Property test for mconcat equivalence to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      heaps in proptest::collection::vec(
        proptest::collection::vec(any::<i32>(), 0..5).prop_map(|v| {
          let heap: BinaryHeap<i32> = v.into_iter().collect();
          heap
        }),
        0..5
      )
    ) {
      let mconcat_result = BinaryHeap::mconcat(heaps.clone());

      let fold_result = heaps.into_iter().fold(
        BinaryHeap::empty(),
        |acc, x| acc.combine(x)
      );

      prop_assert_eq!(
        mconcat_result.into_sorted_vec(),
        fold_result.into_sorted_vec()
      );
    }

    // Property test for combining maintains all elements
    #[test]
    fn prop_combine_preserves_elements(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5)
    ) {
      let heap_a: BinaryHeap<i32> = a.iter().cloned().collect();
      let heap_b: BinaryHeap<i32> = b.iter().cloned().collect();

      let combined = heap_a.clone().combine(heap_b.clone());

      // The combined heap should have all elements from both heaps
      let mut expected = Vec::new();
      expected.extend_from_slice(&a);
      expected.extend_from_slice(&b);
      expected.sort();

      let combined_sorted = combined.into_sorted_vec();

      prop_assert_eq!(combined_sorted, expected);
    }
  }
}
