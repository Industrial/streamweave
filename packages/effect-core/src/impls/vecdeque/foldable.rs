use std::collections::VecDeque;

use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

// Implement Foldable for VecDeque<T>
impl<T: CloneableThreadSafe> Foldable<T> for VecDeque<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    self.into_iter().fold(init, |acc, x| f(acc, &x))
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    let vec: Vec<T> = self.into_iter().collect();
    vec.into_iter().rev().fold(init, |acc, item| f(&item, acc))
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    let mut iter = self.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, val| f(&acc, &val)))
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    let vec: Vec<T> = self.into_iter().collect();
    if vec.is_empty() {
      return None;
    }

    let mut iter = vec.into_iter().rev();
    let last = iter.next().unwrap();
    let result = iter.fold(last, |acc, val| f(&val, &acc));
    Some(result)
  }
}

// Extension trait to make VecDeque foldable operations more ergonomic
pub trait VecDequeFoldableExt<T: CloneableThreadSafe> {
  fn fold<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe;

  fn fold_right<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe;

  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe;

  fn reduce_right<F>(self, f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe;
}

// Implement the extension trait for VecDeque
impl<T: CloneableThreadSafe> VecDequeFoldableExt<T> for VecDeque<T> {
  fn fold<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    self.into_iter().fold(init, |acc, x| f(acc, &x))
  }

  fn fold_right<A, F>(self, init: A, mut f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    let vec: Vec<T> = self.into_iter().collect();
    vec.into_iter().rev().fold(init, |acc, item| f(&item, acc))
  }

  fn reduce<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    let mut iter = self.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, val| f(&acc, &val)))
  }

  fn reduce_right<F>(self, mut f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    let vec: Vec<T> = self.into_iter().collect();
    if vec.is_empty() {
      return None;
    }

    let mut iter = vec.into_iter().rev();
    let last = iter.next().unwrap();
    let result = iter.fold(last, |acc, val| f(&val, &acc));
    Some(result)
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
  fn test_fold_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = Foldable::fold(deque, 0, |acc, _| acc + 1);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_fold_nonempty() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let result = Foldable::fold(deque, 0, |acc, x| acc + x);
    assert_eq!(result, 15);
  }

  #[test]
  fn test_fold_right_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = Foldable::fold_right(deque, 0, |_, acc| acc + 1);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_fold_right_nonempty() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let result = Foldable::fold_right(deque, 0, |x, acc| acc + x);
    assert_eq!(result, 15);
  }

  #[test]
  fn test_reduce_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = Foldable::reduce(deque, |a, b| a + b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_reduce_single() {
    let deque = to_vecdeque(vec![42]);
    let result = Foldable::reduce(deque, |a, b| a + b);
    assert_eq!(result, Some(42));
  }

  #[test]
  fn test_reduce_multiple() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let result = Foldable::reduce(deque, |a, b| a + b);
    assert_eq!(result, Some(15));
  }

  #[test]
  fn test_reduce_right_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = Foldable::reduce_right(deque, |a, b| a + b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_reduce_right_single() {
    let deque = to_vecdeque(vec![42]);
    let result = Foldable::reduce_right(deque, |a, b| a + b);
    assert_eq!(result, Some(42));
  }

  #[test]
  fn test_reduce_right_multiple() {
    let deque = to_vecdeque(vec![1, 2, 3, 4, 5]);
    let result = Foldable::reduce_right(deque, |a, b| a + b);
    assert_eq!(result, Some(15));
  }

  #[test]
  fn test_reduce_right_string() {
    let deque = to_vecdeque(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let result = Foldable::reduce_right(deque, |val, acc| format!("{}{}", acc, val));
    assert_eq!(result, Some("cba".to_string()));
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_fold_identity(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      let deque = to_vecdeque(xs.clone());
      let result = Foldable::fold(deque, 0, |acc, _| acc);
      prop_assert_eq!(result, 0);
    }

    #[test]
    fn prop_fold_sum(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      let deque = to_vecdeque(xs.clone());
      let result = Foldable::fold(deque, 0i32, |acc: i32, x: &i32| acc.saturating_add(*x));
      let expected: i32 = xs.iter().fold(0i32, |acc, x| acc.saturating_add(*x));
      prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_fold_right_sum(xs in prop::collection::vec(any::<i32>(), 0..100)) {
      let deque = to_vecdeque(xs.clone());
      let result = Foldable::fold_right(deque, 0i32, |x: &i32, acc: i32| x.saturating_add(acc));
      let expected: i32 = xs.iter().rev().fold(0i32, |acc, x| acc.saturating_add(*x));
      prop_assert_eq!(result, expected);
    }

    #[test]
    fn prop_reduce_sum(xs in prop::collection::vec(any::<i32>(), 1..100)) {
      let deque = to_vecdeque(xs.clone());
      let result = Foldable::reduce(deque, |a: &i32, b: &i32| a.saturating_add(*b));
      let expected: i32 = xs.iter().fold(0i32, |acc, x| acc.saturating_add(*x));
      prop_assert_eq!(result, Some(expected));
    }

    #[test]
    fn prop_reduce_right_sum(xs in prop::collection::vec(any::<i32>(), 1..100)) {
      let deque = to_vecdeque(xs.clone());
      let result = Foldable::reduce_right(deque, |a: &i32, b: &i32| a.saturating_add(*b));
      let expected: i32 = xs.iter().rev().fold(0i32, |acc, x| acc.saturating_add(*x));
      prop_assert_eq!(result, Some(expected));
    }
  }
}
