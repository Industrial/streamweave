use crate::impls::vecdeque::category::VecDequeCategory;
use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::VecDeque;

// Implement Filterable for VecDequeCategory, not for VecDeque directly
impl<A: CloneableThreadSafe> Filterable<A> for VecDequeCategory {
  type Filtered<B: CloneableThreadSafe> = VecDeque<B>;

  fn filter_map<B, F>(self, _f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    // This is a placeholder implementation since filtering
    // is done through the extension trait
    VecDeque::new()
  }
}

// Extension trait to make VecDeque filtering more ergonomic
pub trait VecDequeFilterableExt<T: CloneableThreadSafe> {
  fn filter_map<B, F>(self, f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;

  fn filter<F>(self, predicate: F) -> VecDeque<T>
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe,
    T: Clone;
}

// Implement the extension trait for VecDeque
impl<T: CloneableThreadSafe> VecDequeFilterableExt<T> for VecDeque<T> {
  fn filter_map<B, F>(mut self, mut f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    let mut result = VecDeque::with_capacity(self.len());
    while let Some(item) = self.pop_front() {
      if let Some(mapped) = f(&item) {
        result.push_back(mapped);
      }
    }
    result
  }

  fn filter<F>(mut self, mut predicate: F) -> VecDeque<T>
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe,
    T: Clone,
  {
    let mut result = VecDeque::with_capacity(self.len());
    while let Some(item) = self.pop_front() {
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

  #[test]
  fn test_filter_map_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = deque.filter_map(f);
    assert_eq!(result, VecDeque::<String>::new());
  }

  #[test]
  fn test_filter_map_all_some() {
    let mut deque = VecDeque::new();
    deque.push_back(20);
    deque.push_back(30);
    deque.push_back(40);
    
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = deque.filter_map(f);
    
    let mut expected = VecDeque::new();
    expected.push_back("20".to_string());
    expected.push_back("30".to_string());
    expected.push_back("40".to_string());
    
    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_map_some_none() {
    let mut deque = VecDeque::new();
    deque.push_back(5);
    deque.push_back(15);
    deque.push_back(25);
    
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = deque.filter_map(f);
    
    let mut expected = VecDeque::new();
    expected.push_back("15".to_string());
    expected.push_back("25".to_string());
    
    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_map_all_none() {
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    
    let f = |x: &i32| if *x > 10 { Some(x.to_string()) } else { None };
    
    let result = deque.filter_map(f);
    assert_eq!(result, VecDeque::<String>::new());
  }

  #[test]
  fn test_filter_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let predicate = |x: &i32| *x > 10;
    
    let result = deque.filter(predicate);
    assert_eq!(result, VecDeque::<i32>::new());
  }

  #[test]
  fn test_filter_all_pass() {
    let mut deque = VecDeque::new();
    deque.push_back(20);
    deque.push_back(30);
    deque.push_back(40);
    
    let predicate = |x: &i32| *x > 10;
    
    let result = deque.clone().filter(predicate);
    assert_eq!(result, deque);
  }

  #[test]
  fn test_filter_some_pass() {
    let mut deque = VecDeque::new();
    deque.push_back(5);
    deque.push_back(15);
    deque.push_back(25);
    
    let predicate = |x: &i32| *x > 10;
    
    let result = deque.filter(predicate);
    
    let mut expected = VecDeque::new();
    expected.push_back(15);
    expected.push_back(25);
    
    assert_eq!(result, expected);
  }

  #[test]
  fn test_filter_none_pass() {
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    
    let predicate = |x: &i32| *x > 10;
    
    let result = deque.filter(predicate);
    assert_eq!(result, VecDeque::<i32>::new());
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_identity_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Identity law: filter_map(Some) == self
      let deque: VecDeque<i32> = xs.clone().into_iter().collect();
      let identity = |val: &i32| Some(*val);
      
      let result: VecDeque<i32> = deque.filter_map(identity);
      let expected: VecDeque<i32> = xs.into_iter().collect();
      
      prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_annihilation_law(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Annihilation law: filter_map(|_| None) == empty
      let deque: VecDeque<i32> = xs.into_iter().collect();
      let none_fn = |_: &i32| None::<i32>;
      
      let result = deque.filter_map(none_fn);
      prop_assert_eq!(result, VecDeque::<i32>::new());
    }

    #[test]
    fn prop_distributivity_law(xs in prop::collection::vec(any::<i32>(), 0..100), limit in 1..100i32) {
      // Distributivity law: filter_map(f).filter_map(g) == filter_map(|x| f(x).and_then(g))
      let deque: VecDeque<i32> = xs.into_iter().collect();
      
      // Define filter functions
      let f = |val: &i32| if val.abs() % 2 == 0 { Some(*val) } else { None };
      let g = move |val: &i32| if val.abs() < limit { Some(val.to_string()) } else { None };
      
      // Apply filters sequentially
      let deque_clone = deque.clone();
      let result1 = deque_clone.filter_map(f).filter_map(g);
      
      // Apply composed filter
      let result2 = deque.filter_map(move |val| {
        f(val).and_then(|v| g(&v))
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_filter_consistent_with_filter_map(xs in prop::collection::vec(any::<i32>(), 0..100), threshold in 1..100i32) {
      // filter(p) == filter_map(|x| if p(x) { Some(x) } else { None })
      let deque: VecDeque<i32> = xs.into_iter().collect();
      let deque_clone = deque.clone();
      
      let predicate = move |val: &i32| val.abs() < threshold;
      
      let result1 = deque.filter(predicate);
      let result2 = deque_clone.filter_map(move |val| {
        if predicate(val) { Some(*val) } else { None }
      });
      
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_preserves_order(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      // Filtering should preserve the order of elements
      let deque: VecDeque<i32> = xs.iter().cloned().collect();
      let predicate = |val: &i32| val % 2 == 0;
      
      let mut manual_result = VecDeque::new();
      for x in xs {
        if predicate(&x) {
          manual_result.push_back(x);
        }
      }
      
      let result = deque.filter(predicate);
      prop_assert_eq!(result, manual_result);
    }
  }
} 