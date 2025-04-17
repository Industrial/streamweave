//! Scannable trait and implementations.
//!
//! A scannable is a type that can perform cumulative operations on its elements,
//! producing intermediate results.

/// The Scannable trait defines operations for performing cumulative operations on elements.
pub trait Scannable<T: 'static> {
  type HigherSelf<U>
  where
    U: 'static;

  /// Performs a left scan (fold) operation, producing all intermediate results
  fn scan_left<B, F>(self, init: B, f: F) -> Self::HigherSelf<B>
  where
    B: 'static,
    F: FnMut(B, T) -> B;

  /// Performs a right scan (fold) operation, producing all intermediate results
  fn scan_right<B, F>(self, init: B, f: F) -> Self::HigherSelf<B>
  where
    B: 'static,
    F: FnMut(T, B) -> B;
}

// Implementation for Option
impl<T: 'static> Scannable<T> for Option<T> {
  type HigherSelf<U>
    = Option<U>
  where
    U: 'static;

  fn scan_left<B, F>(self, init: B, mut f: F) -> Option<B>
  where
    B: 'static,
    F: FnMut(B, T) -> B,
  {
    self.map(|x| f(init, x))
  }

  fn scan_right<B, F>(self, init: B, mut f: F) -> Option<B>
  where
    B: 'static,
    F: FnMut(T, B) -> B,
  {
    self.map(|x| f(x, init))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions for Option
  // These functions are chosen to be:
  // 1. Simple and easy to reason about
  // 2. Cover basic arithmetic operations
  // 3. Include min/max operations for comparison
  // 4. Avoid division to prevent edge cases
  const OPTION_FUNCTIONS: &[fn(i32, i32) -> i32] = &[
    |acc, x| acc + x,    // Addition
    |acc, x| acc * x,    // Multiplication
    |acc, x| acc - x,    // Subtraction
    |acc, x| acc.max(x), // Maximum
    |acc, x| acc.min(x), // Minimum
  ];

  proptest! {
    #[test]
    fn test_option_scan_left_identity(x in any::<i32>()) {
      // Test that scan_left with identity function returns the original value
      let id = |acc: i32, x: i32| x;
      let result = Some(x).scan_left(0, id);
      assert_eq!(result, Some(x));
    }

    #[test]
    fn test_option_scan_left_composition(
      x in any::<i32>(),
      f_idx in 0..OPTION_FUNCTIONS.len(),
      g_idx in 0..OPTION_FUNCTIONS.len()
    ) {
      // Test that composing two functions with scan_left is equivalent to
      // applying the composed function directly
      let f = OPTION_FUNCTIONS[f_idx];
      let g = OPTION_FUNCTIONS[g_idx];
      let result = Some(x).scan_left(0, f).and_then(|y| Some(y).scan_left(0, g));
      let expected = Some(x).scan_left(0, |acc, x| g(f(acc, x), x));
      assert_eq!(result, expected);
    }

    #[test]
    fn test_option_scan_right_identity(x in any::<i32>()) {
      // Test that scan_right with identity function returns the original value
      let id = |x: i32, acc: i32| x;
      let result = Some(x).scan_right(0, id);
      assert_eq!(result, Some(x));
    }

    #[test]
    fn test_option_scan_right_composition(
      x in any::<i32>(),
      f_idx in 0..OPTION_FUNCTIONS.len(),
      g_idx in 0..OPTION_FUNCTIONS.len()
    ) {
      // Test that composing two functions with scan_right is equivalent to
      // applying the composed function directly
      let f = OPTION_FUNCTIONS[f_idx];
      let g = OPTION_FUNCTIONS[g_idx];
      let result = Some(x).scan_right(0, f).and_then(|y| Some(y).scan_right(0, g));
      let expected = Some(x).scan_right(0, |x, acc| g(x, f(x, acc)));
      assert_eq!(result, expected);
    }

    #[test]
    fn test_option_scan_left_none(init in any::<i32>()) {
      // Test that scan_left on None returns None
      let f = |acc: i32, x: i32| acc + x;
      let result = None::<i32>.scan_left(init, f);
      assert_eq!(result, None);
    }

    #[test]
    fn test_option_scan_right_none(init in any::<i32>()) {
      // Test that scan_right on None returns None
      let f = |x: i32, acc: i32| x + acc;
      let result = None::<i32>.scan_right(init, f);
      assert_eq!(result, None);
    }
  }
}
