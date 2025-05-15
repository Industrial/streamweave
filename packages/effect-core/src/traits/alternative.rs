//! Provides the [`Alternative`] trait for types that support choice and repetition in an applicative context.
//!
//! The [`Alternative`] trait is used for types that are both [`Applicative`] and monoidal, supporting operations like `empty` and `alt` for combining values.
//! It is commonly used for types that represent computations with failure or choice, such as `Option` or `Result`.

use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that are both [`Applicative`] and monoidal, supporting choice and repetition.
///
/// The [`Alternative`] trait provides operations for combining values (`alt`) and representing an empty value (`empty`).
///
/// # Laws
///
/// 1. Identity: `alt(empty(), x) == x` and `alt(x, empty()) == x`
/// 2. Associativity: `alt(alt(x, y), z) == alt(x, alt(y, z))`
/// 3. Distributivity for Applicative: `ap(alt(f, g), x) == alt(ap(f, x), ap(g, x))`
/// 4. Distributivity for pure: `alt(pure(x), pure(y)) == pure(alt(x, y))`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type parameter `T` must implement [`CloneableThreadSafe`].
pub trait Alternative<T: CloneableThreadSafe>: Applicative<T> {
  /// Returns the empty value for the type.
  fn empty() -> Self;

  /// Combines two values, choosing the first non-empty one.
  fn alt(self, other: Self) -> Self;
}
