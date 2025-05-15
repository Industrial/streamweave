//! Provides the [`Monoid`] trait for types that form a monoid under an associative operation with an identity element.
//!
//! The [`Monoid`] trait extends [`Semigroup`] by adding an identity element and a method for folding collections of monoid values.
//! It is a fundamental abstraction in algebra and functional programming for combining values.

use crate::traits::semigroup::Semigroup;

/// Trait for types that form a monoid under some operation.
///
/// A monoid is a type with an associative binary operation (see [`Semigroup`]) and an identity element (`empty`).
/// The [`Monoid`] trait also provides a method for folding a collection of monoid values using the monoid operation.
///
/// # Laws
///
/// 1. Identity: `combine(x, empty()) == x` and `combine(empty(), x) == x`
/// 2. Associativity: `combine(combine(x, y), z) == combine(x, combine(y, z))` (from [`Semigroup`])
///
/// # Examples
/// ```
/// use effect_core::traits::monoid::Monoid;
/// let sum = <i32 as Monoid>::mconcat(vec![1, 2, 3]);
/// assert_eq!(sum, 6);
/// ```
pub trait Monoid: Semigroup {
  /// Returns the identity element for the monoid operation.
  fn empty() -> Self;

  /// Folds a collection of monoid values using the monoid operation.
  fn mconcat<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = Self>,
    Self: Sized,
  {
    iter.into_iter().fold(Self::empty(), Self::combine)
  }
}
