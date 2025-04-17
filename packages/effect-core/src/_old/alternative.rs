//! Alternative trait and implementations.
//!
//! The Alternative trait represents types that are both Applicative and Monoid.
//! It provides operations for choice and repetition.

use super::applicative::Applicative;
use std::marker::PhantomData;
use std::sync::Arc;

/// The Alternative trait represents types that are both Applicative and Monoid.
/// It provides operations for choice and repetition.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The functions f and g must implement Send + Sync + 'static
pub trait Alternative<T: Send + Sync + 'static>: Applicative<T> {
  /// The empty value for the Alternative type.
  fn empty() -> Self;

  /// Combines two Alternative values, choosing the first non-empty one.
  fn alt(self, other: Self) -> Self;
}

/// Implementation of Alternative for Option.
impl<T: Send + Sync + 'static> Alternative<T> for Option<T> {
  fn empty() -> Self {
    None
  }

  fn alt(self, other: Self) -> Self {
    self.or(other)
  }
}

/// Implementation of Alternative for Result.
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Alternative<T> for Result<T, E> {
  fn empty() -> Self {
    Err(Arc::new(PhantomData::<E>))
  }

  fn alt(self, other: Self) -> Self {
    self.or(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions that are easy to reason about and won't cause overflow
  const FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x + 1, // Increment
    |x| x * 2, // Double
    |x| x - 1, // Decrement
    |x| x / 2, // Halve
    |x| x * x, // Square
    |x| -x,    // Negate
  ];

  proptest! {
    #[test]
    fn test_option_alternative_identity(x in any::<i32>()) {
      let empty = Option::<i32>::empty();
      assert_eq!(empty, None);

      let some_x = Some(x);
      assert_eq!(some_x.alt(empty), some_x);
      assert_eq!(empty.alt(some_x), some_x);
    }

    #[test]
    fn test_option_alternative_composition(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];

      let some_fx = Some(f(x));
      let some_gx = Some(g(x));

      // Test that alt prefers the first non-empty value
      assert_eq!(some_fx.alt(some_gx), some_fx);
      assert_eq!(some_gx.alt(some_fx), some_gx);
    }

    #[test]
    fn test_result_alternative_identity(x in any::<i32>()) {
      let empty = Result::<i32, &str>::empty();
      assert!(empty.is_err());

      let ok_x = Ok(x);
      assert_eq!(ok_x.alt(empty), ok_x);
      assert_eq!(empty.alt(ok_x), ok_x);
    }

    #[test]
    fn test_result_alternative_composition(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];

      let ok_fx = Ok(f(x));
      let ok_gx = Ok(g(x));

      // Test that alt prefers the first non-empty value
      assert_eq!(ok_fx.alt(ok_gx), ok_fx);
      assert_eq!(ok_gx.alt(ok_fx), ok_gx);
    }

    #[test]
    fn test_option_alternative_empty_composition(
      f_idx in 0..FUNCTIONS.len()
    ) {
      let empty = Option::<i32>::empty();
      let f = FUNCTIONS[f_idx];

      // Test that empty.alt(empty) is empty
      assert_eq!(empty.alt(empty), empty);

      // Test that empty.alt(Some(f(0))) is Some(f(0))
      assert_eq!(empty.alt(Some(f(0))), Some(f(0)));
    }

    #[test]
    fn test_result_alternative_empty_composition(
      f_idx in 0..FUNCTIONS.len()
    ) {
      let empty = Result::<i32, &str>::empty();
      let f = FUNCTIONS[f_idx];

      // Test that empty.alt(empty) is empty
      assert!(empty.alt(empty).is_err());

      // Test that empty.alt(Ok(f(0))) is Ok(f(0))
      assert_eq!(empty.alt(Ok(f(0))), Ok(f(0)));
    }
  }
}
