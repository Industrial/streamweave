use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

impl<T> Monoid for LinkedList<T>
where
  T: CloneableThreadSafe,
{
  fn empty() -> Self {
    LinkedList::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  // Test that empty returns an empty LinkedList
  #[test]
  fn test_empty() {
    let empty_list: LinkedList<i32> = LinkedList::empty();
    assert!(empty_list.is_empty());
  }

  // Test that empty set is the left identity
  #[test]
  fn test_left_identity() {
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);

    let empty_list = LinkedList::empty();
    let result = empty_list.combine(list.clone());
    assert_eq!(result, list);
  }

  // Test that empty set is the right identity
  #[test]
  fn test_right_identity() {
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);

    let empty_list = LinkedList::empty();
    let result = list.clone().combine(empty_list);
    assert_eq!(result, list);
  }

  // Test that combining with an empty list is identity
  #[test]
  fn test_identity_with_empty() {
    let empty_list: LinkedList<i32> = LinkedList::empty();
    let result = empty_list.clone().combine(empty_list.clone());
    assert_eq!(result, LinkedList::empty());
  }

  // Test that mconcat works correctly
  #[test]
  fn test_mconcat() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let mut list2 = LinkedList::new();
    list2.push_back(3);
    list2.push_back(4);

    let mut list3 = LinkedList::new();
    list3.push_back(5);
    list3.push_back(6);

    let result = LinkedList::mconcat(vec![list1, list2, list3]);

    let mut expected = LinkedList::new();
    expected.push_back(1);
    expected.push_back(2);
    expected.push_back(3);
    expected.push_back(4);
    expected.push_back(5);
    expected.push_back(6);

    assert_eq!(result, expected);
  }

  // Test mconcat with empty lists
  #[test]
  fn test_mconcat_all_empty() {
    let empty_vec: Vec<LinkedList<i32>> = vec![];
    let result = LinkedList::mconcat(empty_vec);
    assert_eq!(result, LinkedList::empty());
  }

  // Test mconcat with a mix of empty and non-empty lists
  #[test]
  fn test_mconcat_mixed_empty() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let empty_list: LinkedList<i32> = LinkedList::empty();

    let mut list3 = LinkedList::new();
    list3.push_back(3);
    list3.push_back(4);

    let result = LinkedList::mconcat(vec![list1, empty_list, list3]);

    let mut expected = LinkedList::new();
    expected.push_back(1);
    expected.push_back(2);
    expected.push_back(3);
    expected.push_back(4);

    assert_eq!(result, expected);
  }

  // Test that mconcat is equivalent to folding with combine
  #[test]
  fn test_mconcat_equivalence() {
    let mut list1 = LinkedList::new();
    list1.push_back(1);
    list1.push_back(2);

    let mut list2 = LinkedList::new();
    list2.push_back(3);
    list2.push_back(4);

    let mut list3 = LinkedList::new();
    list3.push_back(5);
    list3.push_back(6);

    let lists = vec![list1.clone(), list2.clone(), list3.clone()];

    let mconcat_result = LinkedList::mconcat(lists.clone());
    let fold_result = lists
      .into_iter()
      .fold(LinkedList::empty(), |acc, x| acc.combine(x));

    assert_eq!(mconcat_result, fold_result);
  }

  // Test associativity with multiple LinkedLists
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

  // Test with different types
  #[test]
  fn test_with_different_types() {
    // Test with strings
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

  // Test with complex type
  #[test]
  fn test_with_complex_type() {
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

    let mut list2 = LinkedList::new();
    list2.push_back(TestStruct {
      id: 2,
      name: "Bob".to_string(),
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

    assert_eq!(result, expected);
  }

  // Property-based tests

  // Test that empty is a left identity
  proptest! {
    #[test]
    fn prop_left_identity(
      elements in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let empty_list = LinkedList::<i32>::empty();
      let result = empty_list.combine(elements.clone());
      prop_assert_eq!(result, elements);
    }
  }

  // Test that empty is a right identity
  proptest! {
    #[test]
    fn prop_right_identity(
      elements in prop::collection::vec(1..100i32, 0..10).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let empty_list = LinkedList::<i32>::empty();
      let result = elements.clone().combine(empty_list);
      prop_assert_eq!(result, elements);
    }
  }

  // Test associativity
  proptest! {
    #[test]
    fn prop_associativity(
      xs in prop::collection::vec(1..100i32, 0..5).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
      ys in prop::collection::vec(1..100i32, 0..5).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
      zs in prop::collection::vec(1..100i32, 0..5).prop_map(|v| v.into_iter().collect::<LinkedList<_>>())
    ) {
      let result1 = xs.clone().combine(ys.clone()).combine(zs.clone());
      let result2 = xs.clone().combine(ys.clone().combine(zs.clone()));
      prop_assert_eq!(result1, result2);
    }
  }

  // Test that empty is consistent
  proptest! {
    #[test]
    fn prop_empty_consistent(_dummy in 0..10) {
      let empty1 = LinkedList::<i32>::empty();
      let empty2 = LinkedList::<i32>::empty();
      prop_assert_eq!(empty1, empty2);
    }
  }

  // Test that mconcat is equivalent to fold
  proptest! {
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      lists in prop::collection::vec(
        prop::collection::vec(1..100i32, 0..5).prop_map(|v| v.into_iter().collect::<LinkedList<_>>()),
        0..5
      )
    ) {
      let mconcat_result = LinkedList::mconcat(lists.clone());
      let fold_result = lists.into_iter().fold(LinkedList::empty(), |acc, x| acc.combine(x));
      prop_assert_eq!(mconcat_result, fold_result);
    }
  }
}
