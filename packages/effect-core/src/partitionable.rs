//! Partitionable trait and implementations.
//!
//! A partitionable is a type that can be split into two parts based on a predicate
//! or a mapping function that produces either left or right values.

use crate::Either;

/// The Partitionable trait defines operations for partitioning elements into two collections.
pub trait Partitionable<T: Send + Sync + 'static> {
  type HigherSelf<U: Send + Sync + 'static>
  where
    U: Send + Sync + 'static;

  /// Partitions elements into two collections based on a predicate.
  /// Elements that satisfy the predicate go into the first collection,
  /// while elements that don't satisfy the predicate go into the second collection.
  fn partition<F>(self, f: F) -> (Self::HigherSelf<T>, Self::HigherSelf<T>)
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static;

  /// Partitions elements into two collections based on a mapping function that
  /// produces either left or right values.
  fn partition_map<L: Send + Sync + 'static, R: Send + Sync + 'static, F>(
    self,
    f: F,
  ) -> (Self::HigherSelf<L>, Self::HigherSelf<R>)
  where
    F: FnMut(T) -> Either<L, R> + Send + Sync + 'static;
}

// Implementation for Option
impl<T: Send + Sync + 'static> Partitionable<T> for Option<T> {
  type HigherSelf<U: Send + Sync + 'static> = Option<U>;

  fn partition<F>(self, mut f: F) -> (Option<T>, Option<T>)
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static,
  {
    match self {
      Some(value) => {
        if f(&value) {
          (Some(value), None)
        } else {
          (None, Some(value))
        }
      }
      None => (None, None),
    }
  }

  fn partition_map<L: Send + Sync + 'static, R: Send + Sync + 'static, F>(
    self,
    mut f: F,
  ) -> (Option<L>, Option<R>)
  where
    F: FnMut(T) -> Either<L, R> + Send + Sync + 'static,
  {
    match self {
      Some(value) => match f(value) {
        Either::Left(l) => (Some(l), None),
        Either::Right(r) => (None, Some(r)),
      },
      None => (None, None),
    }
  }
}

// Implementation for Vec
impl<T: Send + Sync + 'static> Partitionable<T> for Vec<T> {
  type HigherSelf<U: Send + Sync + 'static> = Vec<U>;

  fn partition<F>(self, f: F) -> (Vec<T>, Vec<T>)
  where
    F: FnMut(&T) -> bool + Send + Sync + 'static,
  {
    self.into_iter().partition(f)
  }

  fn partition_map<L: Send + Sync + 'static, R: Send + Sync + 'static, F>(
    self,
    mut f: F,
  ) -> (Vec<L>, Vec<R>)
  where
    F: FnMut(T) -> Either<L, R> + Send + Sync + 'static,
  {
    let mut lefts = Vec::new();
    let mut rights = Vec::new();

    for item in self {
      match f(item) {
        Either::Left(l) => lefts.push(l),
        Either::Right(r) => rights.push(r),
      }
    }

    (lefts, rights)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_option_partition() {
    let a = Some(5);
    assert_eq!(a.partition(|&x| x > 3), (Some(5), None));
    assert_eq!(a.partition(|&x| x < 3), (None, Some(5)));

    let a: Option<i32> = None;
    assert_eq!(a.partition(|&x| x > 3), (None, None));
  }

  #[test]
  fn test_option_partition_map() {
    let a = Some(5);
    let result = a.partition_map(|x| {
      if x > 3 {
        Either::Left(x.to_string())
      } else {
        Either::Right(x * 2)
      }
    });
    assert_eq!(result, (Some("5".to_string()), None));

    let a = Some(2);
    let result = a.partition_map(|x| {
      if x > 3 {
        Either::Left(x.to_string())
      } else {
        Either::Right(x * 2)
      }
    });
    assert_eq!(result, (None, Some(4)));

    let a: Option<i32> = None;
    let result = a.partition_map(|x| {
      if x > 3 {
        Either::Left(x.to_string())
      } else {
        Either::Right(x * 2)
      }
    });
    assert_eq!(result, (None, None));
  }

  #[test]
  fn test_vec_partition() {
    let a = vec![1, 2, 3, 4, 5];
    assert_eq!(a.partition(|&x| x > 3), (vec![4, 5], vec![1, 2, 3]));

    let a: Vec<i32> = vec![];
    assert_eq!(a.partition(|&x| x > 3), (vec![], vec![]));
  }

  #[test]
  fn test_vec_partition_map() {
    let a = vec![1, 2, 3, 4, 5];
    let result = a.partition_map(|x| {
      if x > 3 {
        Either::Left(x.to_string())
      } else {
        Either::Right(x * 2)
      }
    });
    assert_eq!(
      result,
      (vec!["4".to_string(), "5".to_string()], vec![2, 4, 6])
    );

    let a: Vec<i32> = vec![];
    let result = a.partition_map(|x| {
      if x > 3 {
        Either::Left(x.to_string())
      } else {
        Either::Right(x * 2)
      }
    });
    assert_eq!(result, (vec![], vec![]));
  }

  #[test]
  fn test_type_conversions() {
    let a = vec!["hello", "world", "rust"];
    let result = a.partition(|s| s.len() > 4);
    assert_eq!(result, (vec!["hello", "world"], vec!["rust"]));

    let a = vec![1, 2, 3];
    let result = a.partition_map(|x| {
      if x % 2 == 0 {
        Either::Left(x.to_string())
      } else {
        Either::Right(x as f64)
      }
    });
    assert_eq!(result, (vec!["2".to_string()], vec![1.0, 3.0]));
  }
}
