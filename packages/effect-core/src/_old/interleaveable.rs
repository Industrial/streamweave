use std::iter::Iterator;
use std::option::Option;
use std::vec::Vec;

/// A trait for types that can be interleaved with other collections.
pub trait Interleaveable<T> {
  /// Interleaves elements from this collection with another collection.
  /// Returns a new collection containing elements from both collections in an interleaved order.
  fn interleave(self, other: Self) -> Self;

  /// Interleaves elements from this collection with another collection using a function to combine elements.
  /// Returns a new collection containing elements from both collections combined using the provided function.
  fn interleave_with<F, R>(self, other: Self, f: F) -> Self
  where
    F: Fn(T, T) -> R,
    R: Into<T>;
}

impl<T: Clone> Interleaveable<T> for Vec<T> {
  fn interleave(self, other: Vec<T>) -> Vec<T> {
    let mut result = Vec::new();
    let mut iter1 = self.into_iter();
    let mut iter2 = other.into_iter();

    loop {
      match (iter1.next(), iter2.next()) {
        (Some(x), Some(y)) => {
          result.push(x);
          result.push(y);
        }
        (Some(x), None) => result.push(x),
        (None, Some(y)) => result.push(y),
        (None, None) => break,
      }
    }

    result
  }

  fn interleave_with<F, R>(self, other: Vec<T>, f: F) -> Vec<T>
  where
    F: Fn(T, T) -> R,
    R: Into<T>,
  {
    let mut result = Vec::new();
    let mut iter1 = self.into_iter();
    let mut iter2 = other.into_iter();

    loop {
      match (iter1.next(), iter2.next()) {
        (Some(x), Some(y)) => {
          result.push(f(x, y).into());
        }
        (Some(x), None) => result.push(x),
        (None, Some(y)) => result.push(y),
        (None, None) => break,
      }
    }

    result
  }
}

impl<T: Clone> Interleaveable<T> for Option<T> {
  fn interleave(self, other: Option<T>) -> Option<T> {
    match (self, other) {
      (Some(x), Some(_)) => Some(x),
      (Some(x), None) => Some(x),
      (None, Some(y)) => Some(y),
      (None, None) => None,
    }
  }

  fn interleave_with<F, R>(self, other: Option<T>, f: F) -> Option<T>
  where
    F: Fn(T, T) -> R,
    R: Into<T>,
  {
    match (self, other) {
      (Some(x), Some(y)) => Some(f(x, y).into()),
      (Some(x), None) => Some(x),
      (None, Some(y)) => Some(y),
      (None, None) => None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_vec_interleave() {
    let a = vec![1, 3, 5];
    let b = vec![2, 4, 6];
    let result = a.interleave(b);
    assert_eq!(result, vec![1, 2, 3, 4, 5, 6]);

    let a = vec![1, 3];
    let b = vec![2, 4, 6, 8];
    let result = a.interleave(b);
    assert_eq!(result, vec![1, 2, 3, 4, 6, 8]);

    let a = vec![1, 3, 5, 7];
    let b = vec![2, 4];
    let result = a.interleave(b);
    assert_eq!(result, vec![1, 2, 3, 4, 5, 7]);
  }

  #[test]
  fn test_vec_interleave_with() {
    let a = vec![1, 3, 5];
    let b = vec![2, 4, 6];
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, vec![3, 7, 11]);

    let a = vec![1, 3];
    let b = vec![2, 4, 6, 8];
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, vec![3, 7, 6, 8]);

    let a = vec![1, 3, 5, 7];
    let b = vec![2, 4];
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, vec![3, 7, 5, 7]);
  }

  #[test]
  fn test_option_interleave() {
    let a: Option<i32> = Some(1);
    let b: Option<i32> = Some(2);
    let result = a.interleave(b);
    assert_eq!(result, Some(1));

    let a: Option<i32> = Some(1);
    let b: Option<i32> = None;
    let result = a.interleave(b);
    assert_eq!(result, Some(1));

    let a: Option<i32> = None;
    let b: Option<i32> = Some(2);
    let result = a.interleave(b);
    assert_eq!(result, Some(2));

    let a: Option<i32> = None;
    let b: Option<i32> = None;
    let result = a.interleave(b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_option_interleave_with() {
    let a: Option<i32> = Some(1);
    let b: Option<i32> = Some(2);
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, Some(3));

    let a: Option<i32> = Some(1);
    let b: Option<i32> = None;
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, Some(1));

    let a: Option<i32> = None;
    let b: Option<i32> = Some(2);
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, Some(2));

    let a: Option<i32> = None;
    let b: Option<i32> = None;
    let result = a.interleave_with(b, |x, y| x + y);
    assert_eq!(result, None);
  }
}
