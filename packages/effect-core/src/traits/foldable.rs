use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

/// A foldable is a type that can be folded over to reduce its values.
///
/// Foldable types are traversable containers that provide functions to accumulate values.
/// They can be seen as generalizations of sequences that support operations like fold_left, fold_right, and reduce.
///
/// # Laws
///
/// 1. Identity: `fold(foldable, init, |acc, _| acc) == init`
/// 2. Composition: `fold(fold(foldable, init1, f), init2, g) == fold(foldable, g(init2, init1), |acc, x| g(acc, f(init1, x)))`
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement CloneableThreadSafe
/// - The accumulator type A must implement CloneableThreadSafe
/// - The folding function F must implement CloneableThreadSafe
pub trait Foldable<T: CloneableThreadSafe>: Functor<T> {
  /// Folds over the foldable using a binary operation and an initial value
  fn fold<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe;

  /// Folds over the foldable from the right using a binary operation and an initial value
  fn fold_right<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe;

  /// Reduces the foldable using a binary operation
  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe;

  /// Reduces the foldable from the right using a binary operation
  fn reduce_right<F>(self, f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe;
}
