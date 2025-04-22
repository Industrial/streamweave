use crate::traits::foldable::Foldable;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::LinkedList;

use crate::impls::linkedlist::category::LinkedListCategory;

// Implement Foldable for LinkedListCategory, not directly for LinkedList
impl<T: CloneableThreadSafe> Foldable<T> for LinkedListCategory {
  fn fold<A, F>(self, _init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe,
  {
    // This is a placeholder implementation since LinkedListCategory is just a proxy type
    // The actual implementation is in the extension trait
    _init
  }

  fn fold_right<A, F>(self, _init: A, _f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe,
  {
    // This is a placeholder implementation since LinkedListCategory is just a proxy type
    // The actual implementation is in the extension trait
    _init
  }

  fn reduce<F>(self, _f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // This is a placeholder implementation since LinkedListCategory is just a proxy type
    // The actual implementation is in the extension trait
    None
  }

  fn reduce_right<F>(self, _f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe,
  {
    // This is a placeholder implementation since LinkedListCategory is just a proxy type
    // The actual implementation is in the extension trait
    None
  }
}

// Extension trait to make LinkedList foldable operations more ergonomic
pub trait LinkedListFoldableExt<T: CloneableThreadSafe> {
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

// Implement the extension trait for LinkedList
impl<T: CloneableThreadSafe> LinkedListFoldableExt<T> for LinkedList<T> {
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
    // Convert to a Vec to easily iterate in reverse
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

  #[test]
  fn test_fold_empty() {
    let list: LinkedList<i32> = LinkedList::new();
    let result = list.fold(0, |acc, val| acc + val);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_fold_single() {
    let mut list = LinkedList::new();
    list.push_back(5);
    let result = list.fold(0, |acc, val| acc + val);
    assert_eq!(result, 5);
  }

  #[test]
  fn test_fold_multiple() {
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);
    let result = list.fold(0, |acc, val| acc + val);
    assert_eq!(result, 6);
  }

  #[test]
  fn test_fold_right_empty() {
    let list: LinkedList<i32> = LinkedList::new();
    let result = list.fold_right(0, |val, acc| val + acc);
    assert_eq!(result, 0);
  }

  #[test]
  fn test_fold_right_multiple() {
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);
    let result = list.fold_right(0, |val, acc| val + acc);
    assert_eq!(result, 6);
  }

  #[test]
  fn test_fold_right_order() {
    let mut list = LinkedList::new();
    list.push_back("a");
    list.push_back("b");
    list.push_back("c");
    let result = list.fold_right(String::new(), |val, acc| acc + val);
    assert_eq!(result, "cba");
  }

  #[test]
  fn test_reduce_empty() {
    let list: LinkedList<i32> = LinkedList::new();
    let result = list.reduce(|a, b| a + b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_reduce_single() {
    let mut list = LinkedList::new();
    list.push_back(5);
    let result = list.reduce(|a, b| a + b);
    assert_eq!(result, Some(5));
  }

  #[test]
  fn test_reduce_multiple() {
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);
    let result = list.reduce(|a, b| a + b);
    assert_eq!(result, Some(6));
  }

  #[test]
  fn test_reduce_right_empty() {
    let list: LinkedList<i32> = LinkedList::new();
    let result = list.reduce_right(|a, b| a + b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_reduce_right_multiple() {
    let mut list = LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);
    let result = list.reduce_right(|a, b| a + b);
    assert_eq!(result, Some(6));
  }

  #[test]
  fn test_reduce_right_order() {
    let mut list = LinkedList::new();
    list.push_back("a".to_string());
    list.push_back("b".to_string());
    list.push_back("c".to_string());
    let result = list.reduce_right(|a, b| format!("{}{}", b.to_string(), a.to_string()));
    assert_eq!(result, Some("cba".to_string()));
  }

  #[test]
  fn test_with_strings() {
    let mut list: LinkedList<String> = LinkedList::new();
    list.push_back("Hello".to_string());
    list.push_back("World".to_string());

    let result = list.clone().fold(String::new(), |mut acc, s| {
      if !acc.is_empty() {
        acc.push(' ');
      }
      acc.push_str(s);
      acc
    });
    assert_eq!(result, "Hello World");

    let result = list.reduce(|a, b| format!("{} {}", a, b));
    assert_eq!(result, Some("Hello World".to_string()));
  }
}
