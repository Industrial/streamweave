use crate::functor::Functor;
use crate::monad::Monad;
use std::fmt::{Debug, Display};

/// A type that represents an array that is guaranteed to have at least one element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonEmptyArray<T> {
  head: T,
  tail: Vec<T>,
}

impl<T> NonEmptyArray<T> {
  /// Creates a new NonEmptyArray with a single element.
  pub fn single(value: T) -> Self {
    Self {
      head: value,
      tail: Vec::new(),
    }
  }

  /// Creates a new NonEmptyArray from a head and a tail.
  pub fn new(head: T, tail: Vec<T>) -> Self {
    Self { head, tail }
  }

  /// Returns the first element of the array.
  pub fn head(&self) -> &T {
    &self.head
  }

  /// Returns all elements except the first.
  pub fn tail(&self) -> &[T] {
    &self.tail
  }

  /// Returns the length of the array.
  pub fn len(&self) -> usize {
    1 + self.tail.len()
  }

  /// Returns true if the array has only one element.
  pub fn is_single(&self) -> bool {
    self.tail.is_empty()
  }

  /// Converts the NonEmptyArray into a Vec.
  pub fn into_vec(self) -> Vec<T> {
    let mut vec = Vec::with_capacity(1 + self.tail.len());
    vec.push(self.head);
    vec.extend(self.tail);
    vec
  }

  /// Maps a function over all elements of the array.
  pub fn map<U, F>(self, mut f: F) -> NonEmptyArray<U>
  where
    F: FnMut(T) -> U,
  {
    NonEmptyArray {
      head: f(self.head),
      tail: self.tail.into_iter().map(f).collect(),
    }
  }

  /// Folds the array from left to right.
  pub fn fold_left<B, F>(self, init: B, mut f: F) -> B
  where
    F: FnMut(B, T) -> B,
  {
    let mut acc = f(init, self.head);
    for item in self.tail {
      acc = f(acc, item);
    }
    acc
  }
}

impl<T: Send + Sync + 'static> Functor<T> for NonEmptyArray<T> {
  type HigherSelf<U: Send + Sync + 'static> = NonEmptyArray<U>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> B + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    NonEmptyArray {
      head: f(self.head),
      tail: self.tail.into_iter().map(f).collect(),
    }
  }
}

impl<T: Send + Sync + 'static> Monad<T> for NonEmptyArray<T> {
  type HigherSelf<U: Send + Sync + 'static> = NonEmptyArray<U>;

  fn pure(a: T) -> Self::HigherSelf<T> {
    NonEmptyArray::single(a)
  }

  fn bind<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(T) -> Self::HigherSelf<B> + Send + Sync + 'static,
    B: Send + Sync + 'static,
  {
    let mut result = f(self.head);
    for item in self.tail {
      let mut mapped = f(item);
      result.tail.extend(mapped.into_vec());
    }
    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_non_empty_array_creation() {
    let single = NonEmptyArray::single(42);
    assert_eq!(single.head(), &42);
    assert!(single.is_single());
    assert_eq!(single.len(), 1);

    let array = NonEmptyArray::new(1, vec![2, 3, 4]);
    assert_eq!(array.head(), &1);
    assert_eq!(array.tail(), &[2, 3, 4]);
    assert_eq!(array.len(), 4);
  }

  #[test]
  fn test_non_empty_array_map() {
    let array = NonEmptyArray::new(1, vec![2, 3, 4]);
    let mapped = array.map(|x| x * 2);
    assert_eq!(mapped.head(), &2);
    assert_eq!(mapped.tail(), &[4, 6, 8]);
  }

  #[test]
  fn test_non_empty_array_fold_left() {
    let array = NonEmptyArray::new(1, vec![2, 3, 4]);
    let sum = array.fold_left(0, |acc, x| acc + x);
    assert_eq!(sum, 10);
  }

  #[test]
  fn test_functor_laws() {
    // Identity
    let array = NonEmptyArray::new(1, vec![2, 3]);
    let mapped = array.clone().map(|x| x);
    assert_eq!(array, mapped);

    // Composition
    let array = NonEmptyArray::new(1, vec![2, 3]);
    let f = |x: i32| x * 2;
    let g = |x: i32| x + 1;
    let mapped1 = array.clone().map(|x| g(f(x)));
    let mapped2 = array.map(f).map(g);
    assert_eq!(mapped1, mapped2);
  }

  #[test]
  fn test_monad_laws() {
    // Left identity
    let a = 42;
    let f = |x: i32| NonEmptyArray::new(x * 2, vec![x * 3]);
    assert_eq!(NonEmptyArray::pure(a).bind(f), f(a));

    // Right identity
    let m = NonEmptyArray::new(42, vec![43]);
    let m_clone = m.clone();
    assert_eq!(m.bind(NonEmptyArray::pure), m_clone);

    // Associativity
    let m = NonEmptyArray::new(42, vec![43]);
    let m_clone = m.clone();
    let left = m.bind(move |x| {
      let f = |x: i32| NonEmptyArray::new(x * 2, vec![x * 3]);
      let g = |x: i32| NonEmptyArray::new(x + 1, vec![x + 2]);
      f(x).bind(g)
    });
    let right = m_clone
      .bind(|x| NonEmptyArray::new(x * 2, vec![x * 3]))
      .bind(|x| NonEmptyArray::new(x + 1, vec![x + 2]));
    assert_eq!(left, right);
  }
}
