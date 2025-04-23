use crate::traits::applicative::Applicative;
use crate::types::threadsafe::CloneableThreadSafe;

/// The Alternative trait represents types that are both Applicative and Monoid.
/// It provides operations for choice and repetition.
///
/// # Laws
///
/// 1. Identity: `alt(empty(), x) == x` and `alt(x, empty()) == x`
/// 2. Associativity: `alt(alt(x, y), z) == alt(x, alt(y, z))`
/// 3. Distributivity for Applicative: `ap(alt(f, g), x) == alt(ap(f, x), ap(g, x))`
/// 4. Distributivity for pure: `alt(pure(x), pure(y)) == pure(alt(x, y))`
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement CloneableThreadSafe
pub trait Alternative<T: CloneableThreadSafe>: Applicative<T> {
  /// The empty value for the Alternative type.
  fn empty() -> Self;

  /// Combines two Alternative values, choosing the first non-empty one.
  fn alt(self, other: Self) -> Self;
}
