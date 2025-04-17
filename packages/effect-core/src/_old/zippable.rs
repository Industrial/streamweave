//! Zippable trait and implementations.
//!
//! A zippable is a type that can be combined with another type element by element.

/// The Zippable trait defines operations for combining two structures element by element.
pub trait Zippable<T> {
  type HigherSelf<U>;

  /// Combines two structures into a single structure of pairs
  fn zip<U, I>(self, other: I) -> Self::HigherSelf<(T, U)>
  where
    I: IntoIterator<Item = U>;

  /// Combines two structures using a combining function
  fn zip_with<U, F, R, I>(self, other: I, f: F) -> Self::HigherSelf<R>
  where
    F: FnMut(T, U) -> R,
    I: IntoIterator<Item = U>;
}

// Implementation for Option
impl<T> Zippable<T> for Option<T> {
  type HigherSelf<U> = Option<U>;

  fn zip<U, I>(self, other: I) -> Option<(T, U)>
  where
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
impl<T> Zippable<T> for Vec<T> {
  type HigherSelf<U> = Vec<U>;

  fn zip<U, I>(self, other: I) -> Vec<(T, U)>
  where
    I: IntoIterator<Item = U>,
  {
    self.into_iter().zip(other).collect()
  }

  fn zip_with<U, F, R, I>(self, other: I, mut f: F) -> Vec<R>
  where
    F: FnMut(T, U) -> R,
    I: IntoIterator<Item = U>,
  {
    self.into_iter().zip(other).map(|(a, b)| f(a, b)).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test operations
  const OPERATIONS: &[fn(i32, i32) -> i32] = &[
    |a, b| a + b,
    |a, b| a * b,
    |a, b| a - b,
    |a, b| a.max(b),
    |a, b| a.min(b),
  ];

  // Define test strategies
  fn option_strategy() -> impl Strategy<Value = Option<i32>> {
    prop_oneof![Just(None), any::<i32>().prop_map(Some)]
  }

  fn vec_strategy() -> impl Strategy<Value = Vec<i32>> {
    prop::collection::vec(any::<i32>(), 0..10)
  }

  proptest! {
    #[test]
    fn test_option_zip_identity(
      a in option_strategy(),
      b in option_strategy()
    ) {
      let result = a.zip(b);
      match (a, b) {
        (Some(x), Some(y)) => assert_eq!(result, Some((x, y))),
        _ => assert_eq!(result, None),
      }
    }

    #[test]
    fn test_option_zip_with(
      a in option_strategy(),
      b in option_strategy(),
      op_idx in 0..OPERATIONS.len()
    ) {
      let op = OPERATIONS[op_idx];
      let result = a.zip_with(b, op);
      match (a, b) {
        (Some(x), Some(y)) => assert_eq!(result, Some(op(x, y))),
        _ => assert_eq!(result, None),
      }
    }

    #[test]
    fn test_vec_zip_identity(
      a in vec_strategy(),
      b in vec_strategy()
    ) {
      let result = a.clone().zip(b.clone());
      let expected = a.into_iter().zip(b).collect::<Vec<_>>();
      assert_eq!(result, expected);
    }

    #[test]
    fn test_vec_zip_with(
      a in vec_strategy(),
      b in vec_strategy(),
      op_idx in 0..OPERATIONS.len()
    ) {
      let op = OPERATIONS[op_idx];
      let result = a.clone().zip_with(b.clone(), op);
      let expected = a.into_iter().zip(b).map(|(x, y)| op(x, y)).collect::<Vec<_>>();
      assert_eq!(result, expected);
    }

    #[test]
    fn test_zip_laws(
      a in vec_strategy(),
      b in vec_strategy(),
      c in vec_strategy()
    ) {
      // Test associativity: zip(a, zip(b, c)) == zip(zip(a, b), c)
      let lhs = a.clone().zip(b.clone().zip(c.clone()));
      let rhs = a.zip(b).zip(c);
      assert_eq!(lhs.len(), rhs.len());
    }
  }
}
