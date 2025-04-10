//! Functor trait and implementations.
//!
//! A functor is a type that can be mapped over, preserving structure.
//!
//!

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
impl<T> Mappable for Option<T> {}

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
impl<T> Mappable for Vec<T> {}

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

    #[test]
    fn test_complex_type() {
      let some = Some(vec![1, 2, 3]);
      let mapped = some.map(|v| v.into_iter().sum::<i32>());
      assert_eq!(mapped, Some(6));
    }
  }

  mod vec_tests {
    use super::*;

    #[test]
    fn test_empty_vec() {
      let empty: Vec<i32> = vec![];
      let mapped = empty.map(|x| x * 2);
      let expected: Vec<i32> = vec![];
      assert_eq!(mapped, expected);
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

    #[test]
    fn test_complex_type() {
      let vec = vec![vec![1, 2], vec![3, 4]];
      let mapped = vec.map(|v| v.into_iter().sum::<i32>());
      assert_eq!(mapped, vec![3, 7]);
    }

    #[test]
    fn test_mutating_closure() {
      let mut counter = 0;
      let vec = vec![1, 2, 3];
      let mapped = vec.map(|x| {
        counter += 1;
        x * counter
      });
      assert_eq!(mapped, vec![1, 4, 9]);
    }

    #[test]
    fn test_ordering_preservation() {
      let vec = vec![3, 1, 4, 1, 5, 9];
      let mapped = vec.map(|x| x * 2);
      assert_eq!(mapped, vec![6, 2, 8, 2, 10, 18]);
    }
  }
}
