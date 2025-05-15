use crate::impls::linkedlist::category::LinkedListCategory;
use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

// Implement Filterable for LinkedListCategory, not for LinkedList directly
impl<A: CloneableThreadSafe> Filterable<A> for LinkedListCategory {
  type Filtered<B: CloneableThreadSafe> = LinkedList<B>;

  fn filter_map<B, F>(self, _f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is a placeholder implementation since filtering
    // is done through the extension trait
    LinkedList::new()
  }
}

// Extension trait to make LinkedList filtering more ergonomic
pub trait LinkedListFilterableExt<T: CloneableThreadSafe> {
  fn filter_map<B, F>(self, f: F) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;

  fn filter<F>(self, predicate: F) -> LinkedList<T>
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe,
    T: Clone;
}

// Implement the extension trait for LinkedList
impl<T: CloneableThreadSafe> LinkedListFilterableExt<T> for LinkedList<T> {
  fn filter_map<B, F>(self, mut f: F) -> LinkedList<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = LinkedList::new();
    for item in self {
      if let Some(mapped) = f(&item) {
        result.push_back(mapped);
      }
    }
    result
  }

  fn filter<F>(self, mut predicate: F) -> LinkedList<T>
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe,
    T: Clone,
  {
    let mut result = LinkedList::new();
    for item in self {
      if predicate(&item) {
        result.push_back(item);
      }
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper function to create a LinkedList from a Vec
  fn to_linked_list<T: Clone>(vec: Vec<T>) -> LinkedList<T> {
    let mut list = LinkedList::new();
    for item in vec {
      list.push_back(item);
    }
    list
  }

  #[test]
  fn test_filter_map_empty() {
    let list: LinkedList<i32> = LinkedList::new();
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = list.filter_map(f);
    assert_eq!(result, LinkedList::<String>::new());
  }

  #[test]
  fn test_filter_map_all_some() {
    let list = to_linked_list(vec![20, 30, 40]);
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = list.filter_map(f);
    assert_eq!(result, to_linked_list(vec!["20".to_string(), "30".to_string(), "40".to_string()]));
  }

  #[test]
  fn test_filter_map_some_none() {
    let list = to_linked_list(vec![5, 15, 25]);
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = list.filter_map(f);
    assert_eq!(result, to_linked_list(vec!["15".to_string(), "25".to_string()]));
  }

  #[test]
  fn test_filter_map_all_none() {
    let list = to_linked_list(vec![1, 2, 3]);
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = list.filter_map(f);
    assert_eq!(result, LinkedList::<String>::new());
  }

  #[test]
  fn test_filter_empty() {
    let list: LinkedList<i32> = LinkedList::new();
    let predicate = |x: &i32| *x > 10;
    
    let result = list.filter(predicate);
    assert_eq!(result, LinkedList::<i32>::new());
  }

  #[test]
  fn test_filter_all_pass() {
    let list = to_linked_list(vec![20, 30, 40]);
    let predicate = |x: &i32| *x > 10;
    
    let result = list.clone().filter(predicate);
    assert_eq!(result, list);
  }

  #[test]
  fn test_filter_some_pass() {
    let list = to_linked_list(vec![5, 15, 25]);
    let predicate = |x: &i32| *x > 10;
    
    let result = list.filter(predicate);
    assert_eq!(result, to_linked_list(vec![15, 25]));
  }

  #[test]
  fn test_filter_none_pass() {
    let list = to_linked_list(vec![1, 2, 3]);
    let predicate = |x: &i32| *x > 10;
    
    let result = list.filter(predicate);
    assert_eq!(result, LinkedList::<i32>::new());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Identity law: filter_map(Some) == self
      let list = to_linked_list(xs.clone());
      let identity = |val: &i32| Some(*val);
      
      let result = list.filter_map(identity);
      let expected = to_linked_list(xs);
      
      prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_annihilation_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Annihilation law: filter_map(|_| None) == empty
      let list = to_linked_list(xs);
      let none_fn = |_: &i32| None::<i32>;
      
      let result = list.filter_map(none_fn);
      prop_assert_eq!(result, LinkedList::<i32>::new());
    }

    #[test]
    fn prop_distributivity_law(xs in prop::collection::vec(any::<i32>(), 0..100), limit in 1..100i32) {
      // Distributivity law: filter_map(f).filter_map(g) == filter_map(|x| f(x).and_then(g))
      let list = to_linked_list(xs);
      
      // Define filter functions
      let f = |val: &i32| if val.abs() % 2 == 0 { Some(*val) } else { None };
      let g = move |val: &i32| if val.abs() < limit { Some(val.to_string()) } else { None };
      
      // Apply filters sequentially
      let list_clone = list.clone();
      let result1 = list_clone.filter_map(f).filter_map(g);
      
      // Apply composed filter
      let result2 = list.filter_map(move |val| {
        f(val).and_then(|v| g(&v))
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(xs in prop::collection::vec(any::<i32>(), 0..100), threshold in 1..100i32) {
      // filter(p) == filter_map(|x| if p(x) { Some(x) } else { None })
      let list = to_linked_list(xs);
      let list_clone = list.clone();
      
      let predicate = move |val: &i32| val.abs() < threshold;
      
      let result1 = list.filter(predicate);
      let result2 = list_clone.filter_map(move |val| {
        if predicate(val) { Some(*val) } else { None }
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_preserves_order(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Filtering should preserve the order of elements
      let list = to_linked_list(xs.clone());
      let predicate = |val: &i32| val % 2 == 0;
      
      let mut manual_result = LinkedList::new();
      for x in xs {
        if predicate(&x) {
          manual_result.push_back(x);
        }
      }
      
      let result = list.filter(predicate);
      prop_assert_eq!(result, manual_result);
    }
  }
} 