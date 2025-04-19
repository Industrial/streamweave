use crate::traits::semigroup::Semigroup;

/// A trait for types that form a monoid under some operation.
/// A monoid is a type with an associative binary operation and an identity element.
pub trait Monoid: Semigroup {
  /// The identity element for the monoid operation.
  fn empty() -> Self;

  /// Fold a collection of monoid values using the monoid operation.
  fn mconcat<I>(iter: I) -> Self
  where
    I: IntoIterator<Item = Self>,
    Self: Sized,
  {
    iter.into_iter().fold(Self::empty(), Self::combine)
  }
}
