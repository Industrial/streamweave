//! Arrow trait and implementations.
//!
//! An arrow is a generalization of a function that supports arrow operations.
//! It is a descendant of the Category trait and provides additional operations
//! for working with pairs of values.

use super::category::Category;
use super::function::Function;

/// An arrow is a generalization of a function that supports arrow operations.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement Send + Sync + 'static
/// - The type parameter B must implement Send + Sync + 'static
/// - The functions f and g must implement Send + Sync + 'static
pub trait Arrow<A: Send + Sync + 'static, B: Send + Sync + 'static>: Category<(), ()> {
  /// Creates an arrow from a function.
  fn arr<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + Send + Sync + 'static;

  /// Splits an arrow into two parallel arrows.
  fn split<
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    E: Send + Sync + 'static,
    F: Send + Sync + 'static,
  >(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(A, C), (B, D)>;

  /// Combines two arrows into one that operates on pairs.
  fn fanout<C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<A, C>,
  ) -> Self::Morphism<A, (B, C)> {
    Self::split(f, g).compose(Self::arr(|x| (x, x)))
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Arrow<A, B> for Function<A, B> {
  fn arr<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + Send + Sync + 'static,
  {
    Function::new(f)
  }

  fn split<
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    E: Send + Sync + 'static,
    F: Send + Sync + 'static,
  >(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(A, C), (B, D)> {
    Function::new(move |(x, y)| (f.apply(x), g.apply(y)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions
  const FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  proptest! {
    #[test]
    fn test_function_arr(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let arrow = Function::<i32, i32>::arr(f);
      assert_eq!(arrow.apply(x), f(x));
    }

    #[test]
    fn test_function_split(
      x in any::<i32>(),
      y in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = Function::new(FUNCTIONS[f_idx]);
      let g = Function::new(FUNCTIONS[g_idx]);
      let split = Function::split(f, g);
      assert_eq!(split.apply((x, y)), (f.apply(x), g.apply(y)));
    }

    #[test]
    fn test_function_fanout(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = Function::new(FUNCTIONS[f_idx]);
      let g = Function::new(FUNCTIONS[g_idx]);
      let fanout = Function::fanout(f, g);
      assert_eq!(fanout.apply(x), (f.apply(x), g.apply(x)));
    }
  }
}
