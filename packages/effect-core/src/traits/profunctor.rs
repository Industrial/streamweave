//! Provides the [`Profunctor`] trait for types that are contravariant in their first type parameter
//! and covariant in their second type parameter.
//!
//! A profunctor is a generalization of the function arrow, enabling bidirectional mapping
//! over inputs and outputs. It combines aspects of both covariant and contravariant functors.

use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that are contravariant in their first type parameter
/// and covariant in their second type parameter.
///
/// A profunctor allows mapping over both input and output types, with different variance behaviors.
/// It's contravariant in its input type (first parameter) and covariant in its output type (second parameter).
///
/// # Type Parameters
///
/// * `A`: Input type, must implement [`CloneableThreadSafe`].
/// * `B`: Output type, must implement [`CloneableThreadSafe`].
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameters must implement [`CloneableThreadSafe`].
pub trait Profunctor<A: CloneableThreadSafe, B: CloneableThreadSafe>: Sized {
  /// The associated higher-kinded type representation of this profunctor with different type parameters.
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe>: Profunctor<C, D>;

  /// Maps over both input and output types simultaneously.
  ///
  /// This applies a function to the input before this profunctor is applied,
  /// and applies a function to the output after this profunctor is applied.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that maps from type `C` to type `A` (contravariant mapping).
  /// * `g` - A function that maps from type `B` to type `D` (covariant mapping).
  ///
  /// # Returns
  ///
  /// A new profunctor from `C` to `D`.
  fn dimap<C: CloneableThreadSafe, D: CloneableThreadSafe, F, G>(
    self,
    f: F,
    g: G,
  ) -> Self::HigherSelf<C, D>
  where
    F: Fn(C) -> A + CloneableThreadSafe,
    G: Fn(B) -> D + CloneableThreadSafe;

  /// Maps over only the input type (contravariant mapping).
  ///
  /// # Arguments
  ///
  /// * `f` - A function that maps from type `C` to type `A`.
  ///
  /// # Returns
  ///
  /// A new profunctor from `C` to `B`.
  fn lmap<C: CloneableThreadSafe, F>(self, f: F) -> Self::HigherSelf<C, B>
  where
    F: Fn(C) -> A + CloneableThreadSafe,
  {
    self.dimap(f, |x| x)
  }

  /// Maps over only the output type (covariant mapping).
  ///
  /// # Arguments
  ///
  /// * `g` - A function that maps from type `B` to type `D`.
  ///
  /// # Returns
  ///
  /// A new profunctor from `A` to `D`.
  fn rmap<D: CloneableThreadSafe, G>(self, g: G) -> Self::HigherSelf<A, D>
  where
    G: Fn(B) -> D + CloneableThreadSafe,
  {
    self.dimap(|x| x, g)
  }
}
