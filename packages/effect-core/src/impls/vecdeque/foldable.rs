use std::collections::VecDeque;

use crate::impls::vecdeque::category::VecDequeCategory;
use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;

// Implement Foldable for VecDequeCategory, not directly for VecDeque
impl<T: CloneableThreadSafe> Foldable<T> for VecDequeCategory {
  fn fold<A, F>(self, _init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // This is a placeholder implementation since VecDequeCategory is just a proxy type
    // The actual implementation is in the extension trait
    _init
  }

  fn fold_right<A, F>(self, _init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // This is a placeholder implementation since VecDequeCategory is just a proxy type
    // The actual implementation is in the extension trait
    _init
  }

  fn reduce<F>(self, _f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // This is a placeholder implementation since VecDequeCategory is just a proxy type
    // The actual implementation is in the extension trait
    None
  }

  fn reduce_right<F>(self, _f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // This is a placeholder implementation since VecDequeCategory is just a proxy type
    // The actual implementation is in the extension trait
    None
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
  use std::collections::VecDeque;

  #[test]
  fn test_fold_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = deque.fold(0, |acc, val| acc + val);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_fold_single() {
    let mut deque = VecDeque::new();
    deque.push_back(5);
    let result = deque.fold(0, |acc, val| acc + val);
    assert_eq!(result, 5);
  }

  #[test]
  fn test_fold_multiple() {
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    let result = deque.fold(0, |acc, val| acc + val);
    assert_eq!(result, 6);
  }

  #[test]
  fn test_fold_right_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = deque.fold_right(0, |val, acc| acc + val);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_fold_right_multiple() {
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    let result = deque.fold_right(0, |val, acc| acc + val);
    assert_eq!(result, 6);
  }

  #[test]
  fn test_fold_right_order() {
    let mut deque = VecDeque::new();
    deque.push_back("a".to_string());
    deque.push_back("b".to_string());
    deque.push_back("c".to_string());
    let result = deque.fold_right(String::new(), |val, acc| format!("{}{}", acc, val));
    assert_eq!(result, "cba");
  }

  #[test]
  fn test_reduce_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = deque.reduce(|a, b| a + b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_reduce_single() {
    let mut deque = VecDeque::new();
    deque.push_back(5);
    let result = deque.reduce(|a, b| a + b);
    assert_eq!(result, Some(5));
  }

  #[test]
  fn test_reduce_multiple() {
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    let result = deque.reduce(|a, b| a + b);
    assert_eq!(result, Some(6));
  }

  #[test]
  fn test_reduce_right_empty() {
    let deque: VecDeque<i32> = VecDeque::new();
    let result = deque.reduce_right(|a, b| a + b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_reduce_right_multiple() {
    let mut deque = VecDeque::new();
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    let result = deque.reduce_right(|a, b| a + b);
    assert_eq!(result, Some(6));
  }

  #[test]
  fn test_reduce_right_order() {
    let mut deque = VecDeque::new();
    deque.push_back("a".to_string());
    deque.push_back("b".to_string());
    deque.push_back("c".to_string());
    let result = deque.reduce_right(|val, acc| format!("{}{}", acc, val));
    assert_eq!(result, Some("cba".to_string()));
  }
}
