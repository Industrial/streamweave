//! Provides the [`Bifunctor`] trait for types that can be mapped over two type parameters.
//!
//! The [`Bifunctor`] trait generalizes the concept of mapping to types that have two parameters, such as pairs or result types.
//! It is useful for working with data structures that hold two values and need to be transformed independently or together.

use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that can be mapped over two type parameters.
///
/// The [`Bifunctor`] trait provides three main operations:
/// - `bimap`: Maps over both components using two functions
/// - `first`: Maps over the first component
/// - `second`: Maps over the second component
///
/// # Laws
///
/// 1. Identity: `bimap(id, id) = id`
/// 2. Composition: `bimap(f1, g1) . bimap(f2, g2) = bimap(f1 . f2, g1 . g2)`
/// 3. First/Second Commutativity: `first(f) . second(g) = second(g) . first(f)`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameters must implement [`CloneableThreadSafe`].
pub trait Bifunctor<A: CloneableThreadSafe, B: CloneableThreadSafe> {
  /// The higher-kinded type that results from mapping over both values.
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe>: CloneableThreadSafe;

  /// Maps over both values using two different functions.
  fn bimap<C, D, F, G>(self, f: F, g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe;

  /// Maps over the first value only.
  fn first<C, F>(self, f: F) -> Self::HigherSelf<C, B>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe;

  /// Maps over the second value only.
  fn second<D, G>(self, g: G) -> Self::HigherSelf<A, D>
  where
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe;
}
