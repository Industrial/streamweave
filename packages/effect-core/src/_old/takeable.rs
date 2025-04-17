//! Takeable trait and implementations.
//!
//! A takeable is a type that can take or drop elements from its beginning.

/// The Takeable trait defines operations for taking or dropping elements from the beginning.
pub trait Takeable<T> {
  type HigherSelf<U>;

  /// Takes the first n elements from the beginning
  fn take(self, n: usize) -> Self::HigherSelf<T>;

  /// Drops the first n elements from the beginning
  fn drop(self, n: usize) -> Self::HigherSelf<T>;
}

// Implementation for Option
impl<T> Takeable<T> for Option<T> {
  type HigherSelf<U> = Option<U>;

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
impl<T> Takeable<T> for Vec<T> {
  type HigherSelf<U> = Vec<U>;

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
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_option_take_identity(x in any::<i32>()) {
      // Test that taking 1 element from Some(x) returns Some(x)
      let result = Some(x).take(1);
      assert_eq!(result, Some(x));
    }

    #[test]
    fn test_option_take_zero(x in any::<i32>()) {
      // Test that taking 0 elements always returns None
      let result = Some(x).take(0);
      assert_eq!(result, None);
    }

    #[test]
    fn test_option_take_none(n in any::<usize>()) {
      // Test that taking from None always returns None
      let result = None::<i32>.take(n);
      assert_eq!(result, None);
    }

    #[test]
    fn test_option_drop_identity(x in any::<i32>()) {
      // Test that dropping 0 elements from Some(x) returns Some(x)
      let result = Some(x).drop(0);
      assert_eq!(result, Some(x));
    }

    #[test]
    fn test_option_drop_one(x in any::<i32>()) {
      // Test that dropping 1 element from Some(x) returns None
      let result = Some(x).drop(1);
      assert_eq!(result, None);
    }

    #[test]
    fn test_option_drop_none(n in any::<usize>()) {
      // Test that dropping from None always returns None
      let result = None::<i32>.drop(n);
      assert_eq!(result, None);
    }

    #[test]
    fn test_vec_take_length(
      xs in prop::collection::vec(any::<i32>(), 0..100),
      n in any::<usize>()
    ) {
      // Test that take(n) returns a vector of length min(n, xs.len())
      let result = xs.clone().take(n);
      let expected_len = std::cmp::min(n, xs.len());
      assert_eq!(result.len(), expected_len);
    }

    #[test]
    fn test_vec_take_prefix(
      xs in prop::collection::vec(any::<i32>(), 1..100),
      n in 0..100usize
    ) {
      // Test that take(n) returns the first n elements
      let result = xs.clone().take(n);
      let expected = xs.into_iter().take(n).collect::<Vec<_>>();
      assert_eq!(result, expected);
    }

    #[test]
    fn test_vec_drop_length(
      xs in prop::collection::vec(any::<i32>(), 0..100),
      n in any::<usize>()
    ) {
      // Test that drop(n) returns a vector of length max(0, xs.len() - n)
      let result = xs.clone().drop(n);
      let expected_len = xs.len().saturating_sub(n);
      assert_eq!(result.len(), expected_len);
    }

    #[test]
    fn test_vec_drop_suffix(
      xs in prop::collection::vec(any::<i32>(), 1..100),
      n in 0..100usize
    ) {
      // Test that drop(n) returns the elements after the first n
      let result = xs.clone().drop(n);
      let expected = xs.into_iter().skip(n).collect::<Vec<_>>();
      assert_eq!(result, expected);
    }

    #[test]
    fn test_vec_take_drop_composition(
      xs in prop::collection::vec(any::<i32>(), 0..100),
      n in any::<usize>(),
      m in any::<usize>()
    ) {
      // Test that take(n).drop(m) is equivalent to take(min(n, m))
      let result = xs.clone().take(n).drop(m);
      let expected = xs.take(std::cmp::min(n, m));
      assert_eq!(result, expected);
    }
  }
}
