//! Windowable trait and implementations.
//!
//! A windowable is a type that can create sliding windows over its elements.

use std::collections::VecDeque;

/// A trait for types that can create sliding windows.
pub trait Windowable {
  type HigherSelf;

  /// Creates a sliding window of the specified size over the elements.
  /// Returns a vector of windows, where each window is a vector of elements.
  fn window(self, size: usize) -> Vec<Vec<Self::HigherSelf>>
  where
    Self::HigherSelf: Clone;

  /// Creates a sliding window of the specified size over the elements,
  /// applying a function to each window.
  fn window_map<F, R>(self, size: usize, f: F) -> Vec<R>
  where
    F: Fn(&[Self::HigherSelf]) -> R,
    Self::HigherSelf: Clone;
}

// Implementation for Vec
impl<T: Send + Sync + Clone> Windowable for Vec<T> {
  type HigherSelf = T;

  fn window(self, size: usize) -> Vec<Vec<T>>
  where
    T: Clone,
  {
    if size == 0 || size > self.len() {
      return Vec::new();
    }

    let mut windows = Vec::new();
    let mut deque = VecDeque::with_capacity(size);

    for item in self {
      deque.push_back(item.clone());
      if deque.len() == size {
        windows.push(deque.iter().cloned().collect());
        deque.pop_front();
      }
    }

    windows
  }

  fn window_map<F, R>(self, size: usize, f: F) -> Vec<R>
  where
    F: Fn(&[T]) -> R,
    T: Clone,
  {
    if size == 0 || size > self.len() {
      return Vec::new();
    }

    let mut results = Vec::new();
    let mut deque = VecDeque::with_capacity(size);

    for item in self {
      deque.push_back(item.clone());
      if deque.len() == size {
        results.push(f(&deque.make_contiguous()));
        deque.pop_front();
      }
    }

    results
  }
}

// Implementation for Option
impl<T: Clone> Windowable for Option<T> {
  type HigherSelf = T;

  fn window(self, size: usize) -> Vec<Vec<T>>
  where
    T: Clone,
  {
    match self {
      Some(value) if size == 1 => vec![vec![value]],
      _ => Vec::new(),
    }
  }

  fn window_map<F, R>(self, size: usize, f: F) -> Vec<R>
  where
    F: Fn(&[T]) -> R,
    T: Clone,
  {
    match self {
      Some(value) if size == 1 => vec![f(&[value])],
      _ => Vec::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_window() {
    let v = vec![1, 2, 3, 4, 5];
    let windows = v.window(3);
    assert_eq!(windows, vec![vec![1, 2, 3], vec![2, 3, 4], vec![3, 4, 5]]);
  }

  #[test]
  fn test_vec_window_map() {
    let v = vec![1, 2, 3, 4, 5];
    let sums = v.window_map(3, |window| window.iter().sum::<i32>());
    assert_eq!(sums, vec![6, 9, 12]);
  }

  #[test]
  fn test_vec_window_edge_cases() {
    let v = vec![1, 2, 3];
    assert_eq!(v.clone().window(0), Vec::<Vec<i32>>::new());
    assert_eq!(v.clone().window(4), Vec::<Vec<i32>>::new());
    assert_eq!(v.window(3), vec![vec![1, 2, 3]]);
  }

  #[test]
  fn test_option_window() {
    let opt = Some(42);
    assert_eq!(opt.window(1), vec![vec![42]]);
    assert_eq!(opt.window(2), Vec::<Vec<i32>>::new());
    assert_eq!(None::<i32>.window(1), Vec::<Vec<i32>>::new());
  }

  #[test]
  fn test_option_window_map() {
    let opt = Some(42);
    assert_eq!(opt.window_map(1, |w| w[0] * 2), vec![84]);
    assert_eq!(opt.window_map(2, |_| 0), Vec::<i32>::new());
    assert_eq!(None::<i32>.window_map(1, |_| 0), Vec::<i32>::new());
  }
}
