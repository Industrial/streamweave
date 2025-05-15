//! Provides the [`Filterable`] trait for types that support filtering operations.
//!
//! The [`Filterable`] trait is a functional abstraction for types that can selectively
//! include or exclude elements based on a predicate. This extends the concept of [`Functor`]
//! by adding the ability to filter elements during mapping.

use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that support filtering operations.
///
/// The [`Filterable`] trait extends the concept of [`Functor`] by providing operations
/// to selectively include or exclude elements based on predicates or mapping functions.
///
/// # Laws
///
/// 1. Identity preservation: `filterable.filter_map(|x| Some(x)) == filterable`
/// 2. Distributivity: `filterable.filter_map(f).filter_map(g) == filterable.filter_map(|x| f(x).and_then(g))`
/// 3. Annihilation: `filterable.filter_map(|_| None) == empty`
/// 4. Consistency with Functor: `filterable.filter_map(|x| Some(f(x))) == filterable.map(f)`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `A` and all related types must implement [`CloneableThreadSafe`].
pub trait Filterable<A: CloneableThreadSafe>: Functor<A> + Sized {
  /// The result type after filtering operations.
  /// This allows for flexibility in implementations where the filtered result might have a different type.
  type Filtered<B: CloneableThreadSafe>: CloneableThreadSafe;

  /// Maps each element to an Option and keeps only the Some values.
  ///
  /// This is the fundamental operation of the Filterable trait, combining mapping and filtering.
  ///
  /// # Arguments
  ///
  /// * `f` - A function that maps elements to `Option<B>`
  ///
  /// # Returns
  ///
  /// A new filtered container with only the elements where `f` returned `Some`.
  fn filter_map<B, F>(self, f: F) -> Self::Filtered<B>
  where
    F: for<'a> FnMut(&'a A) -> Option<B> + CloneableThreadSafe,
    B: CloneableThreadSafe;

  /// Keeps only the elements that satisfy the predicate.
  ///
  /// # Arguments
  ///
  /// * `predicate` - A function that returns `true` for elements to keep
  ///
  /// # Returns
  ///
  /// A new filtered container with only the elements where `predicate` returned `true`.
  ///
  /// # Default Implementation
  ///
  /// This method has a default implementation in terms of `filter_map`:
  ///
  /// ```
  /// # use std::option::Option;
  /// # trait Filterable<A> {
  /// #     type Filtered<B>;
  /// #     fn filter_map<B, F>(self, f: F) -> Self::Filtered<B>;
  /// #     fn filter<F>(self, predicate: F) -> Self::Filtered<A> where F: FnMut(&A) -> bool;
  /// # }
  /// # impl<A> Filterable<A> for Option<A> {
  /// #     type Filtered<B> = Option<B>;
  /// #     fn filter_map<B, F>(self, _: F) -> Self::Filtered<B> { None }
  /// #     fn filter<F>(self, predicate: F) -> Self::Filtered<A> where F: FnMut(&A) -> bool {
  /// // Default implementation:
  /// let mut predicate = predicate; // Create a mutable binding
  /// self.filter_map(|x| if predicate(x) { Some(x.clone()) } else { None })
  /// #     }
  /// # }
  /// ```
  fn filter<F>(self, predicate: F) -> Self::Filtered<A>
  where
    F: for<'a> FnMut(&'a A) -> bool + CloneableThreadSafe,
    A: Clone,
  {
    let mut predicate = predicate; // Create a mutable binding
    self.filter_map(move |x| if predicate(x) { Some(x.clone()) } else { None })
  }
} 