//! Provides the [`Arrow`] trait for abstracting over composable morphisms with product operations.
//!
//! The [`Arrow`] trait extends [`Category`] with additional operations for splitting and combining arrows,
//! allowing for parallel composition of morphisms and working with product types.
//! It is a powerful abstraction for function composition and working with product types in functional programming.

use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that represent arrows with operations for splitting and combining morphisms.
///
/// The [`Arrow`] trait extends [`Category`] with operations for working with pairs of values,
/// allowing for parallel composition and manipulation of morphisms.
///
/// # Type Parameters
///
/// * `A`: Input type, must implement [`CloneableThreadSafe`].
/// * `B`: Output type, must implement [`CloneableThreadSafe`].
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameters must implement [`CloneableThreadSafe`].
pub trait Arrow<A: CloneableThreadSafe, B: CloneableThreadSafe>: Category<A, B> {
  /// Creates an arrow from a function that consumes its input.
  ///
  /// This differs from the [`Category::arr`] method which takes a function that borrows its input.
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe;

  /// Splits an arrow into two parallel arrows.
  ///
  /// Given two arrows `f: A -> B` and `g: C -> D`, produces an arrow `(A, C) -> (B, D)`
  /// that applies `f` to the first component and `g` to the second component.
  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(A, C), (B, D)>;

  /// Combines two arrows into one that operates on pairs.
  ///
  /// Given two arrows `f: A -> B` and `g: A -> C`, produces an arrow `A -> (B, C)`
  /// that applies both `f` and `g` to the input and returns the pair of results.
  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<A, C>,
  ) -> Self::Morphism<A, (B, C)>;
}
