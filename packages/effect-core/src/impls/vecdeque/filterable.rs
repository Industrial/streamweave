use std::collections::VecDeque;

use crate::traits::filterable::Filterable;
use crate::types::threadsafe::CloneableThreadSafe;

// Implement Filterable for VecDeque<T>
impl<T: CloneableThreadSafe> Filterable<T> for VecDeque<T> {
  type Filtered<B: CloneableThreadSafe> = VecDeque<B>;

  fn filter<F>(self, mut predicate: F) -> Self
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe,
  {
    self.into_iter().filter(|x| predicate(x)).collect()
  }

  fn filter_map<B, F>(self, mut f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    self.into_iter().filter_map(|x| f(&x)).collect()
  }
}

// Extension trait to make VecDeque filterable operations more ergonomic
pub trait VecDequeFilterableExt<T: CloneableThreadSafe> {
  fn filter<F>(self, predicate: F) -> VecDeque<T>
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe;

  fn filter_map<B, F>(self, f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;
}

// Implement the extension trait for VecDeque
impl<T: CloneableThreadSafe> VecDequeFilterableExt<T> for VecDeque<T> {
  fn filter<F>(self, mut predicate: F) -> VecDeque<T>
  where
    F: for<'a> FnMut(&'a T) -> bool + CloneableThreadSafe,
  {
    self.into_iter().filter(|x| predicate(x)).collect()
  }

  fn filter_map<B, F>(self, mut f: F) -> VecDeque<B>
  where
    F: for<'a> FnMut(&'a T) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    self.into_iter().filter_map(|x| f(&x)).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Helper function to convert a Vec to a VecDeque
  fn to_vecdeque<T: Clone>(v: Vec<T>) -> VecDeque<T> {
    v.into_iter().collect()
  }

  #[test]
  fn test_filter_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let predicate = |x: &i32| *x > 0;
    let result = Filterable::filter(deque, predicate);
    assert_eq!(result, VecDeque::new());
  }

  #[test]
  fn test_filter_all_pass() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let predicate = |x: &i32| *x > 0;
    let result = Filterable::filter(deque, predicate);
    assert_eq!(result, to_vecdeque(vec![1, 2, 3, 4, 5]));
  }

  #[test]
  fn test_filter_some_pass() {
    let deque = to_vecdeque(vec![1, -2, 3, -4, 5]);
    let predicate = |x: &i32| *x > 0;
    let result = Filterable::filter(deque, predicate);
    assert_eq!(result, to_vecdeque(vec![1, 3, 5]));
  }

  #[test]
  fn test_filter_none_pass() {
    let deque = to_vecdeque(vec![-1, -2, -3, -4, -5]);
    let predicate = |x: &i32| *x > 0;
    let result = Filterable::filter(deque, predicate);
    assert_eq!(result, VecDeque::new());
  }

  #[test]
  fn test_filter_map_identity() {
    let deque = to_vecdeque(vec![Some(1), Some(2), Some(3)]);
    let identity = |x: &Option<i32>| *x;
    let result: VecDeque<i32> = Filterable::filter_map(deque, identity);
    assert_eq!(result, to_vecdeque(vec![1, 2, 3]));
  }

  #[test]
  fn test_filter_map_with_none() {
    let deque = to_vecdeque(vec![Some(1), None, Some(3)]);
    let identity = |x: &Option<i32>| *x;
    let result = Filterable::filter_map(deque, identity);
    assert_eq!(result, to_vecdeque(vec![1, 3]));
  }

  #[test]
  fn test_filter_map_composition() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let f = |x: &i32| if *x % 2 == 0 { Some(*x * 2) } else { None };
    let g = |x: &i32| if *x > 10 { Some(*x) } else { None };

    let result1 = Filterable::filter_map(Filterable::filter_map(deque.clone(), f), g);
    let result2 = Filterable::filter_map(deque, move |val| {
      let f_result = f(val);
      if let Some(x) = f_result {
        g(&x)
      } else {
        None
      }
    });

    assert_eq!(result1, result2);
  }

  #[test]
  fn test_filter_vs_filter_map() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let predicate = |x: &i32| *x % 2 == 0;

    let result1 = Filterable::filter(deque.clone(), predicate);
    let result2 = Filterable::filter_map(
      deque,
      move |val| {
        if predicate(val) {
          Some(*val)
        } else {
          None
        }
      },
    );

    assert_eq!(result1, result2);
  }

  #[test]
  fn test_filter_with_strings() {
    let deque = to_vecdeque(vec![
      "hello".to_string(),
      "world".to_string(),
      "test".to_string(),
    ]);
    let predicate = |s: &String| s.len() > 4;
    let result = Filterable::filter(deque, predicate);
    assert_eq!(
      result,
      to_vecdeque(vec!["hello".to_string(), "world".to_string()])
    );
  }

  #[test]
  fn test_filter_map_with_strings() {
    let deque = to_vecdeque(vec![
      "hello".to_string(),
      "world".to_string(),
      "test".to_string(),
    ]);
    let f = |s: &String| if s.len() > 4 { Some(s.len()) } else { None };
    let result = Filterable::filter_map(deque, f);
    assert_eq!(result, to_vecdeque(vec![5, 5]));
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_filter_preserves_order(
      xs in prop::collection::vec(any::<i32>(), 0..100)
    ) {
      let deque = to_vecdeque(xs.clone());
      let predicate = |x: &i32| *x > 0;
      let result = Filterable::filter(deque, predicate);

      // Check that the order is preserved
      let expected: Vec<i32> = xs.into_iter().filter(|&x| x > 0).collect();
      prop_assert_eq!(result, to_vecdeque(expected));
    }

    #[test]
    fn prop_filter_map_preserves_order(
      xs in prop::collection::vec(any::<i32>(), 0..100)
    ) {
      let deque = to_vecdeque(xs.clone());
      let f = |x: &i32| if *x > 0 { Some(x.saturating_mul(2)) } else { None };
      let result = Filterable::filter_map(deque, f);

      // Check that the order is preserved
      let expected: Vec<i32> = xs.into_iter().filter_map(|x| if x > 0 { Some(x.saturating_mul(2)) } else { None }).collect();
      prop_assert_eq!(result, to_vecdeque(expected));
    }

    #[test]
    fn prop_filter_empty_result(
      xs in prop::collection::vec(any::<i32>(), 0..100)
    ) {
      let deque = to_vecdeque(xs);
      let predicate = |_: &i32| false;
      let result = Filterable::filter(deque, predicate);
      prop_assert_eq!(result, VecDeque::new());
    }

    #[test]
    fn prop_filter_map_empty_result(
      xs in prop::collection::vec(any::<i32>(), 0..100)
    ) {
      let deque = to_vecdeque(xs);
      let f = |_: &i32| None::<i32>;
      let result = Filterable::filter_map(deque, f);
      prop_assert_eq!(result, VecDeque::new());
    }
  }
}
