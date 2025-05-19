//! Provides the [`Comonad`] trait for types that support extraction and extension.
//!
//! The [`Comonad`] trait is a functional abstraction dual to [`Monad`].
//! Where a monad allows for embedding values and binding computations,
//! a comonad allows for extracting values and extending computations.

use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that support extraction and extension operations.
///
/// The [`Comonad`] trait extends the concept of [`Functor`] by providing operations
/// to extract values and extend computations over the entire structure.
///
/// # Laws
///
/// 1. Left identity: `comonad.extend(|w| w.extract()) == comonad`
/// 2. Right identity: `comonad.extract() == comonad.extend(|w| w.extract()).extract()`
/// 3. Associativity: `comonad.extend(f).extend(g) == comonad.extend(|w| g(w.extend(f)))`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `A` and all related types
/// must implement [`CloneableThreadSafe`].
pub trait Comonad<A: CloneableThreadSafe>: Functor<A> + Sized {
  /// Extracts a value from the comonad.
  ///
  /// This is the fundamental operation that allows retrieving the "current" or "focused"
  /// value from the comonad structure.
  ///
  /// # Returns
  ///
  /// The extracted value of type `A`.
  fn extract(self) -> A;

  /// Extends a computation over the comonad.
  ///
  /// This operation applies a function to the entire comonad structure, transforming it
  /// into another comonad structure containing the results.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that maps from the comonad to a value of type `B`
  ///
  /// # Returns
  ///
  /// A new comonad containing the results of applying the function to the original structure.
  fn extend<B, F>(self, f: F) -> Self::HigherSelf<B>
  where
    F: FnMut(Self) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe;

  /// Maps a function over the comonad, transforming each contained value.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that operates on each value in the comonad
  ///
  /// # Returns
  ///
  /// A new comonad where each value has been transformed by the function.
  ///
  /// # Default Implementation
  ///
  /// This method has a default implementation in terms of `extend` and `extract`:
  ///
  /// ```
  /// # trait Comonad<A> {
  /// #     type HigherSelf<B>;
  /// #     fn extract(self) -> A;
  /// #     fn extend<B, F>(self, f: F) -> Self::HigherSelf<B> where F: FnMut(Self) -> B, Self: Sized;
  /// #     fn duplicate(self) -> Self::HigherSelf<Self> where Self: Sized + Clone + Send + Sync + 'static;
  /// # }
  /// # impl<A> Comonad<A> for Option<A> {
  /// #     type HigherSelf<B> = Option<B>;
  /// #     fn extract(self) -> A { self.unwrap() }
  /// #     fn extend<B, F>(self, _: F) -> Self::HigherSelf<B> where F: FnMut(Self) -> B, Self: Sized { None }
  /// #     fn duplicate(self) -> Self::HigherSelf<Self> where Self: Sized + Clone + Send + Sync + 'static {
  /// // Default implementation:
  /// self.extend(|w| w)
  /// #     }
  /// # }
  /// ```
  fn duplicate(self) -> Self::HigherSelf<Self>
  where
    Self: Sized + Clone + Send + Sync + 'static,
  {
    self.extend(|w| w)
  }
}
