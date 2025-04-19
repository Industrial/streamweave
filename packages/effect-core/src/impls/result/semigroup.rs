use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;

// We need a single implementation for Result<T, E>
// that works for all cases. Similar to Option, we can't have specialized implementations.
//
// The implementation prioritizes Ok values over Err, and for two Ok values,
// it just keeps the first one (basic .or() behavior of Result)

impl<T, E> Semigroup for Result<T, E>
where
  T: Semigroup,
  E: CloneableThreadSafe,
{
  fn combine(self, other: Self) -> Self {
    match (self, other) {
      (Ok(a), Ok(b)) => Ok(a.combine(b)),
      (Ok(a), Err(_)) => Ok(a),
      (Err(_), Ok(b)) => Ok(b),
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

    // Left Ok, Right Err - left value should win
    let a: Result<i32, &str> = Ok(5);
    let b: Result<i32, &str> = Err("error");
    assert_eq!(a.combine(b), Ok(5));

    // Left Err, Right Ok - right value should win
    let a: Result<i32, &str> = Err("error");
    let b: Result<i32, &str> = Ok(10);
    assert_eq!(a.combine(b), Ok(10));

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

    assert_eq!(
      a.clone().combine(b.clone()).combine(c.clone()),
      a.clone().combine(b.clone().combine(c.clone()))
    );

    // Mix of Ok and Err
    let a: Result<i32, &str> = Ok(1);
    let b: Result<i32, &str> = Err("error");
    let c: Result<i32, &str> = Ok(3);

    assert_eq!(
      a.clone().combine(b.clone()).combine(c.clone()),
      a.clone().combine(b.clone().combine(c.clone()))
    );

    // All Err
    let a: Result<i32, &str> = Err("error1");
    let b: Result<i32, &str> = Err("error2");
    let c: Result<i32, &str> = Err("error3");

    assert_eq!(
      a.clone().combine(b.clone()).combine(c.clone()),
      a.clone().combine(b.clone().combine(c.clone()))
    );
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
          ok_a.clone().combine(ok_b.clone()).combine(ok_c.clone()),
          ok_a.clone().combine(ok_b.clone().combine(ok_c.clone()))
        );
      }

      // Test for associativity with mixed Ok and Err
      #[test]
      fn prop_associativity_mixed(a in any::<i32>(), c in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let err_b: Result<i32, &str> = Err("error");
        let ok_c: Result<i32, &str> = Ok(c);

        prop_assert_eq!(
          ok_a.clone().combine(err_b.clone()).combine(ok_c.clone()),
          ok_a.clone().combine(err_b.clone().combine(ok_c.clone()))
        );
      }

      // Test for combine behavior with different result combinations
      #[test]
      fn prop_combine_behavior(a in any::<i32>(), b in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let ok_b: Result<i32, &str> = Ok(b);
        let err: Result<i32, &str> = Err("error");

        // Two Ok values should combine their content - use wrapping_add to avoid overflow
        prop_assert_eq!(ok_a.clone().combine(ok_b.clone()), Ok(a.wrapping_add(b)));

        // Ok + Err = Ok
        prop_assert_eq!(ok_a.clone().combine(err.clone()), ok_a.clone());

        // Err + Ok = Ok
        prop_assert_eq!(err.clone().combine(ok_b.clone()), ok_b.clone());
      }
    }
  }
}
