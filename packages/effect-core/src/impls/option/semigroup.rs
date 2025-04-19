use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;

// We need to define two different versions of Semigroup for Option,
// but we can't have multiple impls for the same type in Rust.
//
// For Option<T> where T is not a Semigroup, we just use .or() (first Some wins)
// For Option<T> where T is a Semigroup, we want to combine the inner values when both are Some
//
// Since we can only have one impl, we'll use a combined implementation
// using a trait bound of CloneableThreadSafe, which is already required for Semigroup
// Specifically, we can't define both impls because they would overlap

impl<T: CloneableThreadSafe> Semigroup for Option<T> {
  fn combine(self, other: Self) -> Self {
    match (self, other) {
      (Some(a), Some(_b)) => Some(a), // Use .or() behavior for non-Semigroup types
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      (None, None) => None,
    }
  }
}

// Unfortunately, we can't specialize for Option<T> where T: Semigroup
// Testing will need to be done using simpler cases and specific types

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::prelude::*;

  // Basic combine tests for Option<T> where T: CloneableThreadSafe
  #[test]
  fn test_option_combine_basic() {
    // None + None = None
    let a: Option<i32> = None;
    let b: Option<i32> = None;
    assert_eq!(a.combine(b), None);

    // Some + None = Some
    let a = Some(1);
    let b: Option<i32> = None;
    assert_eq!(a.combine(b), Some(1));

    // None + Some = Some
    let a: Option<i32> = None;
    let b = Some(2);
    assert_eq!(a.combine(b), Some(2));

    // Some + Some = First Some
    let a = Some(1);
    let b = Some(2);
    assert_eq!(a.combine(b), Some(1));
  }

  // Test using a string type
  #[test]
  fn test_option_combine_string() {
    // None + None = None
    let a: Option<String> = None;
    let b: Option<String> = None;
    assert_eq!(a.combine(b), None);

    // Some + None = Some
    let a = Some("hello".to_string());
    let b: Option<String> = None;
    assert_eq!(a.combine(b), Some("hello".to_string()));

    // None + Some = Some
    let a: Option<String> = None;
    let b = Some("world".to_string());
    assert_eq!(a.combine(b), Some("world".to_string()));

    // Some + Some = First Some
    let a = Some("hello".to_string());
    let b = Some("world".to_string());
    assert_eq!(a.combine(b), Some("hello".to_string()));
  }

  // Test associativity law (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
  #[test]
  fn test_option_associativity() {
    // All None
    test_associativity(None::<i32>, None::<i32>, None::<i32>);

    // Mix of Some and None
    test_associativity(Some(1), None::<i32>, None::<i32>);
    test_associativity(None::<i32>, Some(2), None::<i32>);
    test_associativity(None::<i32>, None::<i32>, Some(3));
    test_associativity(Some(1), Some(2), None::<i32>);
    test_associativity(Some(1), None::<i32>, Some(3));
    test_associativity(None::<i32>, Some(2), Some(3));

    // All Some
    test_associativity(Some(1), Some(2), Some(3));

    // With string values
    test_associativity(
      Some("hello".to_string()),
      Some("world".to_string()),
      Some("!".to_string()),
    );
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_option_associativity(
      a in prop::option::of(1..100i32),
      b in prop::option::of(1..100i32),
      c in prop::option::of(1..100i32)
    ) {
      // For Option, associativity means:
      // (a.combine(b)).combine(c) == a.combine(b.combine(c))
      test_associativity(a, b, c);
    }

    #[test]
    fn prop_option_combine_preserves_values(
      a in prop::option::of(1..100i32),
      b in prop::option::of(1..100i32)
    ) {
      let combined = a.combine(b);

      match (a, b, combined) {
        (Some(val_a), _, Some(result)) => prop_assert_eq!(val_a, result),
        (None, Some(val_b), Some(result)) => prop_assert_eq!(val_b, result),
        (None, None, None) => (), // This is correct
        _ => prop_assert!(false, "Unexpected result from combine"),
      }
    }
  }
}
