//! Functor trait and implementations.
//!
//! A functor is a type that can be mapped over, preserving structure.
//!
//!

use std::iter;

/// Marker trait for types that can be mapped over.
pub trait Mappable {}

/// The Functor trait defines the basic operations for functor types.
pub trait Functor<A> {
  /// The type constructor for the functor.
  type HigherSelf<T>: Functor<T> + Mappable;

  /// Maps a function over the functor.
  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B;
}

// Implement Mappable for Option
impl<A> Mappable for Option<A> {}

// Implementation for Option
impl<A> Functor<A> for Option<A> {
  type HigherSelf<T> = Option<T>;

  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    self.map(f)
  }
}

// Implement Mappable for Vec
impl<A> Mappable for Vec<A> {}

// Implementation for Vec
impl<A> Functor<A> for Vec<A> {
  type HigherSelf<T> = Vec<T>;

  fn map<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(A) -> B,
  {
    self.into_iter().map(f).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  mod option_tests {
    use super::*;

    #[test]
    fn test_some_to_some() {
      let some = Some(42);
      let mapped = some.map(|x| x * 2);
      assert_eq!(mapped, Some(84));
    }

    #[test]
    fn test_none_to_none() {
      let none: Option<i32> = None;
      let mapped = none.map(|x| x * 2);
      assert_eq!(mapped, None);
    }

    #[test]
    fn test_type_conversion() {
      let some = Some(42);
      let mapped = some.map(|x| x.to_string());
      assert_eq!(mapped, Some("42".to_string()));
    }

    #[test]
    fn test_identity() {
      let some = Some(42);
      let mapped = some.map(|x| x);
      assert_eq!(mapped, some);
    }

    #[test]
    fn test_composition() {
      let some = Some(42);
      let mapped = some.map(|x| x * 2).map(|x| x + 1);
      assert_eq!(mapped, Some(85));
    }
  }

  mod vec_tests {
    use super::*;

    #[test]
    fn test_empty_vec() {
      let empty: Vec<i32> = vec![];
      let mapped = empty.map(|x| x * 2);
      assert_eq!(mapped, vec![]);
    }

    #[test]
    fn test_single_element() {
      let vec = vec![42];
      let mapped = vec.map(|x| x * 2);
      assert_eq!(mapped, vec![84]);
    }

    #[test]
    fn test_multiple_elements() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x * 2);
      assert_eq!(mapped, vec![2, 4, 6]);
    }

    #[test]
    fn test_type_conversion() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x.to_string());
      assert_eq!(mapped, vec!["1", "2", "3"]);
    }

    #[test]
    fn test_identity() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x);
      assert_eq!(mapped, vec![1, 2, 3]);
    }

    #[test]
    fn test_composition() {
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| x * 2).map(|x| x + 1);
      assert_eq!(mapped, vec![3, 5, 7]);
    }
  }
}
