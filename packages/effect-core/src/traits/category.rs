//! Provides the [`Category`] trait for abstracting over composable morphisms (arrows) between objects.
//!
//! The [`Category`] trait is a foundational concept in category theory and functional programming, representing a collection of objects and composable morphisms (arrows) between them.
//! It is used as a base for more advanced abstractions like [`Functor`] and [`Applicative`].

use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that represent a category of objects and composable morphisms (arrows) between them.
///
/// The [`Category`] trait provides operations for the identity morphism, composition, and lifting functions to morphisms.
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameters must implement [`CloneableThreadSafe`].
pub trait Category<T: CloneableThreadSafe, U: CloneableThreadSafe> {
  /// The associated morphism type for this category.
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe>: CloneableThreadSafe;

  /// Returns the identity morphism for type `A`.
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A>;

  /// Composes two morphisms: `f: A -> B` and `g: B -> C` to get a morphism `A -> C`.
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>;

  /// Lifts a regular function to a morphism.
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe;

  /// Creates a morphism that applies `f` to the first component of a pair.
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)>;

  /// Creates a morphism that applies `f` to the second component of a pair.
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)>;
}
