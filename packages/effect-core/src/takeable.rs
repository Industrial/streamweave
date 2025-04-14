//! Takeable trait and implementations.
//!
//! A takeable is a type that can take or drop elements from its beginning.

/// The Takeable trait defines operations for taking or dropping elements from the beginning.
pub trait Takeable<T: Send + Sync + 'static> {
  type HigherSelf<U>
  where
    U: Send + Sync + 'static;

  /// Takes the first n elements from the beginning
  fn take(self, n: usize) -> Self::HigherSelf<T>;

  /// Drops the first n elements from the beginning
  fn drop(self, n: usize) -> Self::HigherSelf<T>;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Takeable<T> for Option<T> {
  type HigherSelf<U>
    = Option<U>
  where
    U: Send + Sync + 'static;

  fn take(self, n: usize) -> Option<T> {
    if n == 0 {
      None
    } else {
      self
    }
  }

  fn drop(self, n: usize) -> Option<T> {
    if n == 0 {
      self
    } else {
      None
    }
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Takeable<T> for Vec<T> {
  type HigherSelf<U>
    = Vec<U>
  where
    U: Send + Sync + 'static;

  fn take(self, n: usize) -> Vec<T> {
    self.into_iter().take(n).collect()
  }

  fn drop(self, n: usize) -> Vec<T> {
    self.into_iter().skip(n).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_option_take() {
    let a = Some(5);
    assert_eq!(a.take(0), None);
    assert_eq!(a.take(1), Some(5));
    assert_eq!(a.take(2), Some(5));

    let a: Option<i32> = None;
    assert_eq!(a.take(0), None);
    assert_eq!(a.take(1), None);
  }

  #[test]
  fn test_option_drop() {
    let a = Some(5);
    assert_eq!(a.drop(0), Some(5));
    assert_eq!(a.drop(1), None);
    assert_eq!(a.drop(2), None);

    let a: Option<i32> = None;
    assert_eq!(a.drop(0), None);
    assert_eq!(a.drop(1), None);
  }

  #[test]
  fn test_vec_take() {
    let a = vec![1, 2, 3, 4, 5];
    assert_eq!(a.clone().take(0), vec![]);
    assert_eq!(a.clone().take(1), vec![1]);
    assert_eq!(a.clone().take(3), vec![1, 2, 3]);
    assert_eq!(a.clone().take(5), vec![1, 2, 3, 4, 5]);
    assert_eq!(a.clone().take(10), vec![1, 2, 3, 4, 5]);

    let a: Vec<i32> = vec![];
    assert_eq!(a.clone().take(0), vec![]);
    assert_eq!(a.clone().take(1), vec![]);
  }

  #[test]
  fn test_vec_drop() {
    let a = vec![1, 2, 3, 4, 5];
    assert_eq!(a.clone().drop(0), vec![1, 2, 3, 4, 5]);
    assert_eq!(a.clone().drop(1), vec![2, 3, 4, 5]);
    assert_eq!(a.clone().drop(3), vec![4, 5]);
    assert_eq!(a.clone().drop(5), vec![]);
    assert_eq!(a.clone().drop(10), vec![]);

    let a: Vec<i32> = vec![];
    assert_eq!(a.clone().drop(0), vec![]);
    assert_eq!(a.clone().drop(1), vec![]);
  }

  #[test]
  fn test_type_conversions() {
    let a = vec!["a", "b", "c"];
    let result = a.clone().take(2);
    assert_eq!(result, vec!["a", "b"]);

    let a = vec!["a", "b", "c"];
    let result = a.clone().drop(1);
    assert_eq!(result, vec!["b", "c"]);
  }
}
