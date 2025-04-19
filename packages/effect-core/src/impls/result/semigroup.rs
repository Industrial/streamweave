use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;

// We need a single implementation for Result<T, E>
// that works for all cases. Similar to Option, we can't have specialized implementations.
//
// The implementation prioritizes Err values over Ok, and for two Ok values,
// it combines them using the Semigroup implementation of T.

impl<T, E> Semigroup for Result<T, E>
where
  T: Semigroup,
  E: CloneableThreadSafe,
{
  fn combine(self, other: Self) -> Self {
    match (self, other) {
      (Ok(a), Ok(b)) => Ok(a.combine(b)),
      (Ok(_), Err(e)) => Err(e),
      (Err(e), Ok(_)) => Err(e),
      (Err(e), Err(_)) => Err(e),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Basic combine test
  #[test]
  fn test_combine() {
    // Both Ok - values should be combined
    let a: Result<i32, &str> = Ok(5);
    let b: Result<i32, &str> = Ok(10);
    assert_eq!(a.combine(b), Ok(15));

    // Left Ok, Right Err - right value (Err) should win
    let a: Result<i32, &str> = Ok(5);
    let b: Result<i32, &str> = Err("error");
    assert_eq!(a.combine(b), Err("error"));

    // Left Err, Right Ok - left value (Err) should win
    let a: Result<i32, &str> = Err("error");
    let b: Result<i32, &str> = Ok(10);
    assert_eq!(a.combine(b), Err("error"));

    // Both Err - first error should win
    let a: Result<i32, &str> = Err("error1");
    let b: Result<i32, &str> = Err("error2");
    assert_eq!(a.combine(b), Err("error1"));
  }

  // Test with integers
  #[test]
  fn test_combine_ints() {
    let a: Result<i32, &str> = Ok(5);
    let b: Result<i32, &str> = Ok(10);
    assert_eq!(a.combine(b), Ok(15));
  }

  // Test associativity: (a.combine(b)).combine(c) = a.combine(b.combine(c))
  #[test]
  fn test_result_associativity() {
    // All Ok
    let a: Result<i32, &str> = Ok(1);
    let b: Result<i32, &str> = Ok(2);
    let c: Result<i32, &str> = Ok(3);

    assert_eq!(a.combine(b).combine(c), a.combine(b.combine(c)));

    // Mix of Ok and Err
    let a: Result<i32, &str> = Ok(1);
    let b: Result<i32, &str> = Err("error");
    let c: Result<i32, &str> = Ok(3);

    assert_eq!(a.combine(b).combine(c), a.combine(b.combine(c)));

    // All Err
    let a: Result<i32, &str> = Err("error1");
    let b: Result<i32, &str> = Err("error2");
    let c: Result<i32, &str> = Err("error3");

    assert_eq!(a.combine(b).combine(c), a.combine(b.combine(c)));
  }

  // Test with bool
  #[test]
  fn test_combine_bool() {
    let a: Result<bool, i32> = Ok(true);
    let b: Result<bool, i32> = Ok(false);
    assert_eq!(a.combine(b), Ok(true));
  }

  // Property-based tests
  #[cfg(test)]
  mod prop_tests {
    use super::*;

    proptest! {
      // Test for associativity
      #[test]
      fn prop_associativity(a in any::<i32>(), b in any::<i32>(), c in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let ok_b: Result<i32, &str> = Ok(b);
        let ok_c: Result<i32, &str> = Ok(c);

        prop_assert_eq!(
          ok_a.combine(ok_b).combine(ok_c),
          ok_a.combine(ok_b.combine(ok_c))
        );
      }

      // Test for associativity with mixed Ok and Err
      #[test]
      fn prop_associativity_mixed(a in any::<i32>(), c in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let err_b: Result<i32, &str> = Err("error");
        let ok_c: Result<i32, &str> = Ok(c);

        prop_assert_eq!(
          ok_a.combine(err_b).combine(ok_c),
          ok_a.combine(err_b.combine(ok_c))
        );
      }

      // Test for combine behavior with different result combinations
      #[test]
      fn prop_combine_behavior(a in any::<i32>(), b in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let ok_b: Result<i32, &str> = Ok(b);
        let err: Result<i32, &str> = Err("error");

        // Two Ok values should combine their content - use wrapping_add to avoid overflow
        prop_assert_eq!(ok_a.combine(ok_b), Ok(a.wrapping_add(b)));

        // Ok + Err = Err
        prop_assert_eq!(ok_a.combine(err), err);

        // Err + Ok = Err
        prop_assert_eq!(err.combine(ok_b), err);
      }
    }
  }
}
