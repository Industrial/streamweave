use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::BinaryHeap;

impl<T> Semigroup for BinaryHeap<T>
where
  T: Ord + CloneableThreadSafe,
{
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test with empty heaps
  #[test]
  fn test_combine_empty_heaps() {
    let heap1: BinaryHeap<i32> = BinaryHeap::new();
    let heap2: BinaryHeap<i32> = BinaryHeap::new();

    let result = heap1.combine(heap2);
    assert!(result.is_empty());
  }

  // Test combining empty and non-empty heaps
  #[test]
  fn test_combine_empty_with_nonempty() {
    let empty: BinaryHeap<i32> = BinaryHeap::new();

    let mut non_empty = BinaryHeap::new();
    non_empty.push(1);
    non_empty.push(2);
    non_empty.push(3);

    // Empty + Non-empty
    let result1 = empty.clone().combine(non_empty.clone());
    assert_eq!(result1.len(), 3);
    assert_eq!(result1.into_sorted_vec(), vec![1, 2, 3]);

    // Non-empty + Empty
    let result2 = non_empty.combine(empty);
    assert_eq!(result2.len(), 3);
    assert_eq!(result2.into_sorted_vec(), vec![1, 2, 3]);
  }

  // Test combining two non-empty heaps
  #[test]
  fn test_combine_non_empty_heaps() {
    let mut heap1 = BinaryHeap::new();
    heap1.push(1);
    heap1.push(3);
    heap1.push(5);

    let mut heap2 = BinaryHeap::new();
    heap2.push(2);
    heap2.push(4);
    heap2.push(6);

    let combined = heap1.combine(heap2);
    assert_eq!(combined.len(), 6);
    assert_eq!(combined.into_sorted_vec(), vec![1, 2, 3, 4, 5, 6]);
  }

  // Test with duplicate elements
  #[test]
  fn test_combine_with_duplicates() {
    let mut heap1 = BinaryHeap::new();
    heap1.push(1);
    heap1.push(2);
    heap1.push(3);

    let mut heap2 = BinaryHeap::new();
    heap2.push(2);
    heap2.push(3);
    heap2.push(4);

    let combined = heap1.combine(heap2);
    assert_eq!(combined.len(), 6); // BinaryHeap keeps duplicates
    assert_eq!(combined.into_sorted_vec(), vec![1, 2, 2, 3, 3, 4]);
  }

  // Test max-heap property is preserved
  #[test]
  fn test_max_heap_property() {
    let mut heap1 = BinaryHeap::new();
    heap1.push(1);
    heap1.push(3);
    heap1.push(5);

    let mut heap2 = BinaryHeap::new();
    heap2.push(2);
    heap2.push(4);
    heap2.push(6);

    let mut combined = heap1.combine(heap2);

    // Elements should be popped in descending order (max first)
    assert_eq!(combined.pop(), Some(6));
    assert_eq!(combined.pop(), Some(5));
    assert_eq!(combined.pop(), Some(4));
    assert_eq!(combined.pop(), Some(3));
    assert_eq!(combined.pop(), Some(2));
    assert_eq!(combined.pop(), Some(1));
    assert_eq!(combined.pop(), None);
  }

  // Test using a custom associativity check that works with BinaryHeap
  #[test]
  fn test_associativity_with_vectors() {
    let mut heap_a = BinaryHeap::new();
    heap_a.push(1);
    heap_a.push(4);

    let mut heap_b = BinaryHeap::new();
    heap_b.push(2);
    heap_b.push(5);

    let mut heap_c = BinaryHeap::new();
    heap_c.push(3);
    heap_c.push(6);

    // (a + b) + c
    let ab = heap_a.clone().combine(heap_b.clone());
    let ab_c = ab.combine(heap_c.clone());

    // a + (b + c)
    let bc = heap_b.clone().combine(heap_c.clone());
    let a_bc = heap_a.clone().combine(bc);

    // Convert to sorted vectors for comparison
    assert_eq!(ab_c.into_sorted_vec(), a_bc.into_sorted_vec());
  }

  // Test all permutations of three heaps
  #[test]
  fn test_all_permutations() {
    let mut heap_a = BinaryHeap::new();
    heap_a.push(1);

    let mut heap_b = BinaryHeap::new();
    heap_b.push(2);

    let mut heap_c = BinaryHeap::new();
    heap_c.push(3);

    // All possible permutations:
    // (a + b) + c
    let ab_c_vec = heap_a
      .clone()
      .combine(heap_b.clone())
      .combine(heap_c.clone())
      .into_sorted_vec();

    // (a + c) + b
    let ac_b_vec = heap_a
      .clone()
      .combine(heap_c.clone())
      .combine(heap_b.clone())
      .into_sorted_vec();

    // (b + a) + c
    let ba_c_vec = heap_b
      .clone()
      .combine(heap_a.clone())
      .combine(heap_c.clone())
      .into_sorted_vec();

    // (b + c) + a
    let bc_a_vec = heap_b
      .clone()
      .combine(heap_c.clone())
      .combine(heap_a.clone())
      .into_sorted_vec();

    // (c + a) + b
    let ca_b_vec = heap_c
      .clone()
      .combine(heap_a.clone())
      .combine(heap_b.clone())
      .into_sorted_vec();

    // (c + b) + a
    let cb_a_vec = heap_c
      .clone()
      .combine(heap_b.clone())
      .combine(heap_a.clone())
      .into_sorted_vec();

    // a + (b + c)
    let a_bc_vec = heap_a
      .clone()
      .combine(heap_b.clone().combine(heap_c.clone()))
      .into_sorted_vec();

    // a + (c + b)
    let a_cb_vec = heap_a
      .clone()
      .combine(heap_c.clone().combine(heap_b.clone()))
      .into_sorted_vec();

    // b + (a + c)
    let b_ac_vec = heap_b
      .clone()
      .combine(heap_a.clone().combine(heap_c.clone()))
      .into_sorted_vec();

    // b + (c + a)
    let b_ca_vec = heap_b
      .clone()
      .combine(heap_c.clone().combine(heap_a.clone()))
      .into_sorted_vec();

    // c + (a + b)
    let c_ab_vec = heap_c
      .clone()
      .combine(heap_a.clone().combine(heap_b.clone()))
      .into_sorted_vec();

    // c + (b + a)
    let c_ba_vec = heap_c.combine(heap_b.combine(heap_a)).into_sorted_vec();

    // All should be equal
    let expected = vec![1, 2, 3];
    assert_eq!(ab_c_vec, expected);
    assert_eq!(ac_b_vec, expected);
    assert_eq!(ba_c_vec, expected);
    assert_eq!(bc_a_vec, expected);
    assert_eq!(ca_b_vec, expected);
    assert_eq!(cb_a_vec, expected);
    assert_eq!(a_bc_vec, expected);
    assert_eq!(a_cb_vec, expected);
    assert_eq!(b_ac_vec, expected);
    assert_eq!(b_ca_vec, expected);
    assert_eq!(c_ab_vec, expected);
    assert_eq!(c_ba_vec, expected);
  }

  // Proptest-based tests
  proptest! {
    // Test associativity property
    #[test]
    fn prop_associativity(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5),
      c in proptest::collection::vec(any::<i32>(), 0..5)
    ) {
      let heap_a: BinaryHeap<i32> = a.into_iter().collect();
      let heap_b: BinaryHeap<i32> = b.into_iter().collect();
      let heap_c: BinaryHeap<i32> = c.into_iter().collect();

      let left = heap_a.clone().combine(heap_b.clone()).combine(heap_c.clone());
      let right = heap_a.combine(heap_b.combine(heap_c));

      prop_assert_eq!(left.into_sorted_vec(), right.into_sorted_vec());
    }

    // Test that combining preserves all elements
    #[test]
    fn prop_combines_all_elements(
      a in proptest::collection::vec(any::<i32>(), 0..10),
      b in proptest::collection::vec(any::<i32>(), 0..10)
    ) {
      let heap_a: BinaryHeap<i32> = a.iter().cloned().collect();
      let heap_b: BinaryHeap<i32> = b.iter().cloned().collect();

      let combined = heap_a.combine(heap_b);
      let result_vec = combined.into_sorted_vec();

      // Create a sorted vector with all elements from a and b
      let mut expected = a.clone();
      expected.extend(b);
      expected.sort();

      prop_assert_eq!(result_vec, expected);
    }

    // Test that the combined size is the sum of the original sizes
    #[test]
    fn prop_combine_size(
      a in proptest::collection::vec(any::<i32>(), 0..10),
      b in proptest::collection::vec(any::<i32>(), 0..10)
    ) {
      let heap_a: BinaryHeap<i32> = a.iter().cloned().collect();
      let heap_b: BinaryHeap<i32> = b.iter().cloned().collect();

      let combined = heap_a.combine(heap_b);

      prop_assert_eq!(combined.len(), a.len() + b.len());
    }

    // Test that the max-heap property is preserved
    #[test]
    fn prop_max_heap_property(
      a in proptest::collection::vec(any::<i32>(), 0..5),
      b in proptest::collection::vec(any::<i32>(), 0..5)
    ) {
      let heap_a: BinaryHeap<i32> = a.into_iter().collect();
      let heap_b: BinaryHeap<i32> = b.into_iter().collect();

      let mut combined = heap_a.combine(heap_b);

      // Verify max-heap property if not empty
      if !combined.is_empty() {
        let mut prev = combined.pop().unwrap();

        while let Some(next) = combined.pop() {
          prop_assert!(prev >= next);
          prev = next;
        }
      }
    }
  }
}
