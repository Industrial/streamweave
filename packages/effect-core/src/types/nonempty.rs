//! Provides the [`NonEmpty`] type for collections that are guaranteed to have at least one element.
//!
//! The [`NonEmpty`] type is a wrapper around collection types that guarantees
//! the collection has at least one element, enabling operations that would be unsafe
//! or require error handling on potentially empty collections.

use std::iter::FromIterator;
use std::ops::Index;

/// A wrapper around a collection type that guarantees it has at least one element.
///
/// This is particularly useful for implementing [`Comonad`] on collection types,
/// as comonads require the ability to extract a value, which is not safe for empty collections.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NonEmpty<T> {
  /// The first element of the collection.
  pub head: T,
  /// The rest of the elements in the collection.
  pub tail: Vec<T>,
}

impl<T> NonEmpty<T> {
  /// Creates a new [`NonEmpty`] collection with a single element.
  ///
  /// # Arguments
  ///
  /// * `head` - The element to be stored in the collection
  ///
  /// # Returns
  ///
  /// A new [`NonEmpty`] collection containing only the provided element
  pub fn new(head: T) -> Self {
    NonEmpty {
      head,
      tail: Vec::new(),
    }
  }

  /// Creates a new [`NonEmpty`] collection with a head element and an existing tail vector.
  ///
  /// # Arguments
  ///
  /// * `head` - The first element of the collection
  /// * `tail` - A vector containing the rest of the elements
  ///
  /// # Returns
  ///
  /// A new [`NonEmpty`] collection containing the head followed by all elements in tail
  pub fn from_parts(head: T, tail: Vec<T>) -> Self {
    NonEmpty { head, tail }
  }

  /// Converts a vector into a [`NonEmpty`] collection.
  ///
  /// # Arguments
  ///
  /// * `vec` - A vector with at least one element
  ///
  /// # Returns
  ///
  /// A [`NonEmpty`] collection containing all elements from the vector
  ///
  /// # Panics
  ///
  /// Panics if the provided vector is empty.
  pub fn from_vec(mut vec: Vec<T>) -> Self {
    if vec.is_empty() {
      panic!("Cannot create NonEmpty from an empty vector");
    }

    let head = vec.remove(0);
    NonEmpty { head, tail: vec }
  }

  /// Safely attempts to convert a vector into a [`NonEmpty`] collection.
  ///
  /// # Arguments
  ///
  /// * `vec` - A vector that may be empty
  ///
  /// # Returns
  ///
  /// `Some(NonEmpty)` if the vector had at least one element, `None` otherwise
  pub fn from_vec_safe(mut vec: Vec<T>) -> Option<Self> {
    if vec.is_empty() {
      None
    } else {
      let head = vec.remove(0);
      Some(NonEmpty { head, tail: vec })
    }
  }

  /// Returns the number of elements in the collection.
  ///
  /// # Returns
  ///
  /// The number of elements (always at least 1)
  pub fn len(&self) -> usize {
    1 + self.tail.len()
  }

  /// Converts the [`NonEmpty`] collection into a standard vector.
  ///
  /// # Returns
  ///
  /// A vector containing all elements in the collection
  pub fn into_vec(self) -> Vec<T> {
    let mut result = vec![self.head];
    result.extend(self.tail);
    result
  }

  /// Returns a reference to the element at the specified index.
  ///
  /// # Arguments
  ///
  /// * `index` - The index of the element to retrieve
  ///
  /// # Returns
  ///
  /// A reference to the element at the specified index
  ///
  /// # Panics
  ///
  /// Panics if the index is out of bounds.
  pub fn get(&self, index: usize) -> &T {
    if index == 0 {
      &self.head
    } else {
      &self.tail[index - 1]
    }
  }

  /// Returns a mutable reference to the element at the specified index.
  ///
  /// # Arguments
  ///
  /// * `index` - The index of the element to retrieve
  ///
  /// # Returns
  ///
  /// A mutable reference to the element at the specified index
  ///
  /// # Panics
  ///
  /// Panics if the index is out of bounds.
  pub fn get_mut(&mut self, index: usize) -> &mut T {
    if index == 0 {
      &mut self.head
    } else {
      &mut self.tail[index - 1]
    }
  }

  /// Maps a function over each element in the collection.
  ///
  /// # Arguments
  ///
  /// * `f` - A function to apply to each element
  ///
  /// # Returns
  ///
  /// A new [`NonEmpty`] collection with the function applied to each element
  pub fn map<F, U>(&self, mut f: F) -> NonEmpty<U>
  where
    F: FnMut(&T) -> U,
  {
    let head = f(&self.head);
    let tail = self.tail.iter().map(f).collect();
    NonEmpty { head, tail }
  }

  /// Returns an iterator over references to the elements in the collection.
  pub fn iter(&self) -> impl Iterator<Item = &T> {
    std::iter::once(&self.head).chain(self.tail.iter())
  }

  /// Returns an iterator over mutable references to the elements in the collection.
  pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
    std::iter::once(&mut self.head).chain(self.tail.iter_mut())
  }
}

impl<T> Index<usize> for NonEmpty<T> {
  type Output = T;

  fn index(&self, index: usize) -> &Self::Output {
    self.get(index)
  }
}

impl<T> FromIterator<T> for NonEmpty<T> {
  fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
    let mut iter = iter.into_iter();

    match iter.next() {
      Some(head) => {
        let tail: Vec<T> = iter.collect();
        NonEmpty { head, tail }
      }
      None => panic!("Cannot create NonEmpty from an empty iterator"),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_creation() {
    let ne = NonEmpty::new(1);
    assert_eq!(ne.head, 1);
    assert_eq!(ne.tail.len(), 0);

    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);
    assert_eq!(ne.head, 1);
    assert_eq!(ne.tail, vec![2, 3, 4]);

    let ne = NonEmpty::from_vec(vec![1, 2, 3]);
    assert_eq!(ne.head, 1);
    assert_eq!(ne.tail, vec![2, 3]);
  }

  #[test]
  #[should_panic]
  fn test_from_vec_empty() {
    let _ne = NonEmpty::<i32>::from_vec(vec![]);
  }

  #[test]
  fn test_from_vec_safe() {
    let ne = NonEmpty::from_vec_safe(vec![1, 2, 3]);
    assert!(ne.is_some());
    let ne = ne.unwrap();
    assert_eq!(ne.head, 1);
    assert_eq!(ne.tail, vec![2, 3]);

    let ne = NonEmpty::<i32>::from_vec_safe(vec![]);
    assert!(ne.is_none());
  }

  #[test]
  fn test_len() {
    let ne = NonEmpty::new(1);
    assert_eq!(ne.len(), 1);

    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);
    assert_eq!(ne.len(), 4);
  }

  #[test]
  fn test_into_vec() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);
    let vec = ne.into_vec();
    assert_eq!(vec, vec![1, 2, 3, 4]);
  }

  #[test]
  fn test_indexing() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);
    assert_eq!(ne[0], 1);
    assert_eq!(ne[1], 2);
    assert_eq!(ne[2], 3);
    assert_eq!(ne[3], 4);
  }

  #[test]
  fn test_map() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);
    let mapped = ne.map(|x| x * 2);
    assert_eq!(mapped.head, 2);
    assert_eq!(mapped.tail, vec![4, 6, 8]);
  }

  #[test]
  fn test_iter() {
    let ne = NonEmpty::from_parts(1, vec![2, 3, 4]);
    let collected: Vec<i32> = ne.iter().cloned().collect();
    assert_eq!(collected, vec![1, 2, 3, 4]);
  }

  #[test]
  fn test_from_iterator() {
    let ne: NonEmpty<i32> = vec![1, 2, 3, 4].into_iter().collect();
    assert_eq!(ne.head, 1);
    assert_eq!(ne.tail, vec![2, 3, 4]);
  }

  #[test]
  #[should_panic]
  fn test_from_iterator_empty() {
    let _ne: NonEmpty<i32> = Vec::<i32>::new().into_iter().collect();
  }
}
