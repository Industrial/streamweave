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
  ///
  /// This uses Generic Associated Types (GATs) for better type safety and performance.
  /// The associated type must be a proper higher-kinded type that preserves the functor structure.
  type HigherSelf<U: CloneableThreadSafe>: CloneableThreadSafe + Functor<U>;

  /// Maps a function over the functor, transforming its contents while preserving structure.
  ///
  /// This method should be implemented to provide zero-cost abstractions where possible.
  /// The function `f` should be called exactly once per element in the functor.
  fn map<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;

  /// Maps a function over the functor, transforming its contents while preserving structure.
  /// This is a more efficient version that takes ownership of the function.
  ///
  /// # Performance
  ///
  /// This method should be preferred over `map` when the function can be consumed,
  /// as it avoids the overhead of cloning the function.
  fn map_owned<U, F>(self, f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized;
}
