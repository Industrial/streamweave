use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

impl<T> Semigroup for LinkedList<T>
where
  T: CloneableThreadSafe,
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

  // Test basic combine functionality
  #[test]
  fn test_combine_basic() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let mut list2 = LinkedList::new();
    list2.push_back(3);
    list2.push_back(4);

    let result = list1.combine(list2);

    let mut expected = LinkedList::new();
    expected.push_back(1);
    expected.push_back(2);
    expected.push_back(3);
    expected.push_back(4);

    assert_eq!(result, expected);
  }

  // Test combining with empty lists
  #[test]
  fn test_combine_empties() {
    let empty1: LinkedList<i32> = LinkedList::new();
    let empty2: LinkedList<i32> = LinkedList::new();

    let result = empty1.combine(empty2);
    assert!(result.is_empty());
  }

  // Test combining empty with non-empty
  #[test]
  fn test_combine_with_empty() {
    let empty: LinkedList<i32> = LinkedList::new();

    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);

    // Empty + Non-empty
    let result1 = empty.clone().combine(list.clone());
    assert_eq!(result1, list);

    // Non-empty + Empty
    let result2 = list.clone().combine(empty);
    assert_eq!(result2, list);
  }

  // Test preserving order of elements
  #[test]
  fn test_combine_preserves_order() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let mut list2 = LinkedList::new();
    list2.push_back(3);
    list2.push_back(4);

    let result = list1.combine(list2);

    // Create an iterator to verify the order of elements
    let mut iter = result.iter();
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), Some(&4));
    assert_eq!(iter.next(), None);
  }

  // Test associativity property
  #[test]
  fn test_associativity() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let mut list2 = LinkedList::new();
    list2.push_back(3);
    list2.push_back(4);

    let mut list3 = LinkedList::new();
    list3.push_back(5);
    list3.push_back(6);

    // (list1 combine list2) combine list3
    let result1 = list1.clone().combine(list2.clone()).combine(list3.clone());

    // list1 combine (list2 combine list3)
    let result2 = list1.clone().combine(list2.clone().combine(list3.clone()));

    assert_eq!(result1, result2);
  }

  // Test all combination permutations
  #[test]
  fn test_all_combine_permutations() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let mut list2 = LinkedList::new();
    list2.push_back(3);
    list2.push_back(4);

    let mut list3 = LinkedList::new();
    list3.push_back(5);
    list3.push_back(6);

    // Different ways to associate three lists (but keeping the order)
    let result1 = list1.clone().combine(list2.clone()).combine(list3.clone());
    let result2 = list1.clone().combine(list2.clone().combine(list3.clone()));

    // Expected result: [1, 2, 3, 4, 5, 6]
    let mut expected = LinkedList::new();
    expected.push_back(1);
    expected.push_back(2);
    expected.push_back(3);
    expected.push_back(4);
    expected.push_back(5);
    expected.push_back(6);

    assert_eq!(result1, expected);
    assert_eq!(result2, expected);
  }

  // Test with string type
  #[test]
  fn test_with_string_type() {
    let mut list1 = LinkedList::new();
    list1.push_back("hello".to_string());
    list1.push_back("world".to_string());

    let mut list2 = LinkedList::new();
    list2.push_back("foo".to_string());
    list2.push_back("bar".to_string());

    let result = list1.combine(list2);

    let mut expected = LinkedList::new();
    expected.push_back("hello".to_string());
    expected.push_back("world".to_string());
    expected.push_back("foo".to_string());
    expected.push_back("bar".to_string());

    assert_eq!(result, expected);
  }

  // Test with non-copy custom type
  #[test]
  fn test_with_custom_type() {
    #[derive(Debug, PartialEq, Eq, Clone)]
    struct TestStruct {
      id: i32,
      name: String,
    }

    let mut list1 = LinkedList::new();
    list1.push_back(TestStruct {
      id: 1,
      name: "Alice".to_string(),
    });
    list1.push_back(TestStruct {
      id: 2,
      name: "Bob".to_string(),
    });

    let mut list2 = LinkedList::new();
    list2.push_back(TestStruct {
      id: 3,
      name: "Charlie".to_string(),
    });
    list2.push_back(TestStruct {
      id: 4,
      name: "Dave".to_string(),
    });

    let result = list1.combine(list2);

    let mut expected = LinkedList::new();
    expected.push_back(TestStruct {
      id: 1,
      name: "Alice".to_string(),
    });
    expected.push_back(TestStruct {
      id: 2,
      name: "Bob".to_string(),
    });
    expected.push_back(TestStruct {
      id: 3,
      name: "Charlie".to_string(),
    });
    expected.push_back(TestStruct {
      id: 4,
      name: "Dave".to_string(),
    });

    assert_eq!(result, expected);
  }

  // Property-based tests

  // Test associativity property
  proptest! {
    #[test]
    fn prop_associativity(
      xs in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
      ys in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
      zs in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let result1 = xs.clone().combine(ys.clone()).combine(zs.clone());
      let result2 = xs.clone().combine(ys.clone().combine(zs.clone()));
      prop_assert_eq!(result1, result2);
    }
  }

  // Test that combine preserves all elements in the right order
  proptest! {
    #[test]
    fn prop_combine_preserves_elements_and_order(
      xs in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
      ys in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let combined = xs.clone().combine(ys.clone());

      // Convert lists to vectors for easier comparison
      let xs_vec: Vec<_> = xs.iter().collect();
      let ys_vec: Vec<_> = ys.iter().collect();
      let combined_vec: Vec<_> = combined.iter().collect();

      // First part should be xs
      for (i, x) in xs_vec.iter().enumerate() {
        prop_assert_eq!(combined_vec[i], *x);
      }

      // Second part should be ys
      for (i, y) in ys_vec.iter().enumerate() {
        prop_assert_eq!(combined_vec[xs_vec.len() + i], *y);
      }

      // Length should be sum of both
      prop_assert_eq!(combined_vec.len(), xs_vec.len() + ys_vec.len());
    }
  }

  // Test that the size of the combined list is correct
  proptest! {
    #[test]
    fn prop_combine_size(
      xs in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
      ys in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let combined = xs.clone().combine(ys.clone());

      // Count elements
      let xs_len = xs.len();
      let ys_len = ys.len();

      prop_assert_eq!(combined.len(), xs_len + ys_len);
    }
  }
}
