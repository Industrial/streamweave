//! Provides the [`Functor`] trait for types that can be mapped over, preserving structure.
//!
//! The [`Functor`] trait is a foundational abstraction in functional programming, representing types that support a `map` operation.
//! It is used as a base for more advanced abstractions like [`Applicative`] and [`Foldable`].

use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that can be mapped over, preserving their structure.
///
/// The [`Functor`] trait provides the `map` operation, which transforms the contents of the functor while preserving its structure.
///
/// # Laws
///
/// 1. Identity: `functor.map(|x| x) == functor`
/// 2. Composition: `functor.map(|x| g(f(x))) == functor.map(f).map(g)`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `T` and all related types must implement [`CloneableThreadSafe`].
pub trait Functor<T: CloneableThreadSafe>: Category<T, T> {
  /// The higher-kinded type that results from mapping over this functor.
  type HigherSelf<U: CloneableThreadSafe>: CloneableThreadSafe;

  /// Maps a function over the functor, transforming its contents while preserving structure.
  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}
