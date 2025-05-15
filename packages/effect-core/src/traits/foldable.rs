//! Provides the [`Foldable`] trait for types that can be folded to reduce their values.
//!
//! The [`Foldable`] trait generalizes the concept of folding or reducing a collection to a single value using an accumulator function.
//! It is commonly used for traversable containers and supports operations like `fold`, `fold_right`, and `reduce`.

use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that can be folded over to reduce their values.
///
/// The [`Foldable`] trait provides methods for left and right folds, as well as reductions using binary operations.
///
/// # Laws
///
/// 1. Identity: `fold(foldable, init, |acc, _| acc) == init`
/// 2. Composition: `fold(fold(foldable, init1, f), init2, g) == fold(foldable, g(init2, init1), |acc, x| g(acc, f(init1, x)))`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `T` and all related types must implement [`CloneableThreadSafe`].
pub trait Foldable<T: CloneableThreadSafe>: Functor<T> {
  /// Folds over the foldable using a binary operation and an initial value.
  fn fold<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(A, &'a T) -> A + CloneableThreadSafe;

  /// Folds over the foldable from the right using a binary operation and an initial value.
  fn fold_right<A, F>(self, init: A, f: F) -> A
  where
    A: CloneableThreadSafe,
    F: for<'a> FnMut(&'a T, A) -> A + CloneableThreadSafe;

  /// Reduces the foldable using a binary operation.
  fn reduce<F>(self, f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe;

  /// Reduces the foldable from the right using a binary operation.
  fn reduce_right<F>(self, f: F) -> Option<T>
  where
    F: for<'a, 'b> FnMut(&'a T, &'b T) -> T + CloneableThreadSafe;
}
