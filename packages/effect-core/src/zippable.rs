//! Zippable trait and implementations.
//!
//! A zippable is a type that can be combined with another type element by element.

/// The Zippable trait defines operations for combining two structures element by element.
pub trait Zippable<T: Send + Sync + 'static> {
  type HigherSelf<U>
  where
    U: Send + Sync + 'static;

  /// Combines two structures into a single structure of pairs
  fn zip<U, I>(self, other: I) -> Self::HigherSelf<(T, U)>
  where
    U: Send + Sync + 'static,
    I: IntoIterator<Item = U>;

  /// Combines two structures using a combining function
  fn zip_with<U, F, R, I>(self, other: I, f: F) -> Self::HigherSelf<R>
  where
    U: Send + Sync + 'static,
    R: Send + Sync + 'static,
    F: FnMut(T, U) -> R,
    I: IntoIterator<Item = U>;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Zippable<T> for Option<T> {
  type HigherSelf<U>
    = Option<U>
  where
    U: Send + Sync + 'static;

  fn zip<U, I>(self, other: I) -> Option<(T, U)>
  where
    U: Send + Sync + 'static,
    I: IntoIterator<Item = U>,
  {
    let mut other = other.into_iter();
    match (self, other.next()) {
      (Some(a), Some(b)) => Some((a, b)),
      _ => None,
    }
  }

  fn zip_with<U, F, R, I>(self, other: I, mut f: F) -> Option<R>
  where
    U: Send + Sync + 'static,
    R: Send + Sync + 'static,
    F: FnMut(T, U) -> R,
    I: IntoIterator<Item = U>,
  {
    let mut other = other.into_iter();
    match (self, other.next()) {
      (Some(a), Some(b)) => Some(f(a, b)),
      _ => None,
    }
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Zippable<T> for Vec<T> {
  type HigherSelf<U>
    = Vec<U>
  where
    U: Send + Sync + 'static;

  fn zip<U, I>(self, other: I) -> Vec<(T, U)>
  where
    U: Send + Sync + 'static,
    I: IntoIterator<Item = U>,
  {
    self.into_iter().zip(other).collect()
  }

  fn zip_with<U, F, R, I>(self, other: I, mut f: F) -> Vec<R>
  where
    U: Send + Sync + 'static,
    R: Send + Sync + 'static,
    F: FnMut(T, U) -> R,
    I: IntoIterator<Item = U>,
  {
    self.into_iter().zip(other).map(|(a, b)| f(a, b)).collect()
  }
}

#[cfg(test)]
mod tests {
  use crate::zippable::Zippable;
  use proptest::prelude::*;

  #[test]
  fn test_option_zip() {
    let a = Some(1);
    let b = Some("hello");
    let result = a.zip(b);
    assert_eq!(result, Some((1, "hello")));

    let a: Option<i32> = None;
    let b = Some("hello");
    let result = a.zip(b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_option_zip_with() {
    let a = Some(2);
    let b = Some(3);
    let result = a.zip_with(b, |x, y| x * y);
    assert_eq!(result, Some(6));

    let a: Option<i32> = None;
    let b = Some(3);
    let result = a.zip_with(b, |x, y| x * y);
    assert_eq!(result, None);
  }

  #[test]
  fn test_vec_zip() {
    let a = vec![1, 2, 3];
    let b = vec!["a", "b", "c"];
    let result = a.zip(b);
    assert_eq!(result, vec![(1, "a"), (2, "b"), (3, "c")]);

    let a = vec![1, 2, 3];
    let b = vec!["a", "b"];
    let result = a.zip(b);
    assert_eq!(result, vec![(1, "a"), (2, "b")]);
  }

  #[test]
  fn test_vec_zip_with() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];
    let result = a.zip_with(b, |x, y| x * y);
    assert_eq!(result, vec![4, 10, 18]);

    let a = vec![1, 2, 3];
    let b = vec![4, 5];
    let result = a.zip_with(b, |x, y| x * y);
    assert_eq!(result, vec![4, 10]);
  }

  #[test]
  fn test_type_conversions() {
    let a = vec![1, 2, 3];
    let b = vec![4, 5, 6];
    let result = a.zip_with(b, |x, y| format!("{}:{}", x, y));
    assert_eq!(result, vec!["1:4", "2:5", "3:6"]);
  }

  #[test]
  fn test_empty_collections() {
    let a: Vec<i32> = vec![];
    let b = vec![1, 2, 3];
    let result = a.zip(b);
    assert_eq!(result, Vec::<(i32, i32)>::new());

    let a: Option<i32> = None;
    let b: Option<i32> = None;
    let result = a.zip(b);
    assert_eq!(result, None);
  }

  #[test]
  fn test_zippable_laws() {
    proptest!(|(x: i32, y: i32)| {
      let a = vec![x];
      let b = vec![y];
      let result = Zippable::zip_with(a, b, |x, y| x * y);
      assert_eq!(result, vec![x * y]);

      let a = vec![x, y];
      let b = vec![x, y];
      let result = Zippable::zip_with(a, b, |x, y| x * y);
      assert_eq!(result, vec![x * x, y * y]);
    });
  }
}
