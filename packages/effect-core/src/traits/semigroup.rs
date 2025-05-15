//! Provides the [`Semigroup`] trait for types that form a semigroup under an associative binary operation.
//!
//! The [`Semigroup`] trait is a fundamental algebraic abstraction for types that can be combined using an associative operation.
//! It is used as a base for more advanced abstractions like [`Monoid`].

use crate::types::threadsafe::CloneableThreadSafe;

/// Trait for types that form a semigroup under some operation.
///
/// A semigroup is a type with an associative binary operation (`combine`).
///
/// # Laws
///
/// 1. Associativity: `combine(combine(x, y), z) == combine(x, combine(y, z))`
///
/// # Thread Safety
///
/// All implementations must be thread-safe. The type must implement [`CloneableThreadSafe`].
pub trait Semigroup: CloneableThreadSafe {
  /// Combines two values using the semigroup operation.
  fn combine(self, other: Self) -> Self;
}

#[cfg(test)]
pub mod tests {
  use super::*;

  // Define test functions with overflow protection for numeric types
  pub const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Helper function to test associativity
  pub fn test_associativity<T>(a: T, b: T, c: T)
  where
    T: Semigroup + Clone + PartialEq + std::fmt::Debug,
  {
    // (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
    let left = a.clone().combine(b.clone()).combine(c.clone());
    let right = a.combine(b.combine(c));

    assert_eq!(left, right, "Associativity law failed");
  }

  // A macro to define property-based tests for associativity
  #[macro_export]
  macro_rules! proptest_semigroup_associativity {
    ($test_name:ident, $type:ty, $strategy:expr) => {
      proptest! {
        #[test]
        fn $test_name(a in $strategy.clone(), b in $strategy.clone(), c in $strategy) {
          $crate::traits::semigroup::tests::test_associativity(a, b, c);
        }
      }
    };
  }
}
