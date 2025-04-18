use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;

/// A functor is a type that can be mapped over, preserving structure.
///
/// # Laws
///
/// 1. Identity: `functor.map(|x| x) == functor`
/// 2. Composition: `functor.map(|x| g(f(x))) == functor.map(f).map(g)`
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement CloneableThreadSafe
/// - The higher-kinded type HigherSelf<U> must implement CloneableThreadSafe
/// - The mapping function F must implement CloneableThreadSafe
/// - The mapped type U must implement CloneableThreadSafe
pub trait Functor<T: CloneableThreadSafe>: Category<T, T> {
  /// The higher-kinded type that results from mapping over this functor
  type HigherSelf<U: CloneableThreadSafe>: CloneableThreadSafe;

  /// Maps a function over the functor
  ///
  /// This transforms the contents of the functor while preserving its structure.
  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::functor::Functor;
  use std::fmt::Debug;

  // Testing helper function that verifies the identity law
  fn test_identity_law<F, T>(functor: F, expected: F::HigherSelf<T>)
  where
    F: Functor<T>,
    T: CloneableThreadSafe + PartialEq + Debug + Clone,
    F::HigherSelf<T>: PartialEq + Debug,
  {
    let id = |x: &T| x.clone();
    let result = functor.map(id);
    assert_eq!(result, expected);
  }

  // Note: The composition law should be tested directly in each implementation
  // rather than through a helper function due to complex type constraints
}
