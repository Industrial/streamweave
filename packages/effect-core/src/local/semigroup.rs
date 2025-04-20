//! Local (non-thread-safe) Semigroup implementations.
//!
//! This module contains Semigroup-like functionality for types that
//! cannot implement the Semigroup trait due to thread-safety requirements.

use std::rc::Rc;

/// A trait for non-thread-safe types that have Semigroup-like behavior.
///
/// This is a local version of the Semigroup trait for types that cannot
/// implement Send + Sync (such as Rc<T>).
pub trait LocalSemigroup: Clone + 'static {
  /// Combines two values using the semigroup operation.
  fn local_combine(self, other: Self) -> Self;
}

/// Implementation of LocalSemigroup for Rc<i32>.
impl LocalSemigroup for Rc<i32> {
  fn local_combine(self, other: Self) -> Self {
    Rc::new((*self).wrapping_add(*other))
  }
}

/// Implementation of LocalSemigroup for Rc<String>.
impl LocalSemigroup for Rc<String> {
  fn local_combine(self, other: Self) -> Self {
    let mut result = (*self).clone();
    result.push_str(&*other);
    Rc::new(result)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_combine_basic() {
    // Test with i32
    let a = Rc::new(5);
    let b = Rc::new(10);
    let result = a.local_combine(b);
    assert_eq!(*result, 15);
  }

  #[test]
  fn test_combine_strings() {
    // Test with String
    let a = Rc::new(String::from("Hello, "));
    let b = Rc::new(String::from("world!"));
    let result = a.local_combine(b);
    assert_eq!(*result, "Hello, world!");
  }

  #[test]
  fn test_associativity() {
    // Test associativity manually
    let a = Rc::new(1);
    let b = Rc::new(2);
    let c = Rc::new(3);

    let left = a.clone().local_combine(b.clone()).local_combine(c.clone());
    let right = a.local_combine(b.local_combine(c));

    assert_eq!(*left, *right);
    assert_eq!(*left, 6); // 1 + 2 + 3 = 6
  }

  // Macro to define property-based tests for associativity
  macro_rules! proptest_local_semigroup_associativity {
    ($test_name:ident, $type:ty, $strategy:expr) => {
      proptest! {
          #[test]
          fn $test_name(a in $strategy.clone(), b in $strategy.clone(), c in $strategy) {
              // Test associativity: (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
              let left = a.clone().local_combine(b.clone()).local_combine(c.clone());
              let right = a.local_combine(b.local_combine(c));

              // Compare the Rc pointers by comparing their inner values
              // For primitive types we can compare directly
              assert_eq!(left.as_ref(), right.as_ref());
          }
      }
    };
  }

  proptest_local_semigroup_associativity!(
    prop_rc_i32_associativity,
    Rc<i32>,
    any::<i32>().prop_map(Rc::new)
  );

  proptest_local_semigroup_associativity!(
    prop_rc_string_associativity,
    Rc<String>,
    any::<String>().prop_map(|s| Rc::new(s.chars().take(10).collect::<String>()))
  );

  proptest! {
      #[test]
      fn prop_rc_preserves_inner_value(a in any::<i32>(), b in any::<i32>()) {
          let rc_a = Rc::new(a);
          let rc_b = Rc::new(b);
          let result = rc_a.local_combine(rc_b);

          // The combined rc value should equal the sum of the inner values using wrapping_add
          prop_assert_eq!(*result, a.wrapping_add(b));
      }

      #[test]
      fn prop_rc_string_preserves_inner_value(
          a in any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
          b in any::<String>().prop_map(|s| s.chars().take(5).collect::<String>())
      ) {
          let rc_a = Rc::new(a.clone());
          let rc_b = Rc::new(b.clone());
          let result = rc_a.local_combine(rc_b);

          // The combined rc value should equal the concatenation of the inner values
          let expected = a + &b;
          prop_assert_eq!(result.as_ref(), &expected);
      }
  }
}
