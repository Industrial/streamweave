use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T, E> Monoid for Result<T, E>
where
  T: Monoid,
  E: CloneableThreadSafe,
{
  fn empty() -> Self {
    Ok(T::empty())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;

  #[test]
  fn test_empty() {
    let empty_result: Result<i32, &str> = Monoid::empty();
    assert_eq!(empty_result, Ok(0));

    let empty_result: Result<bool, &str> = Monoid::empty();
    assert_eq!(empty_result, Ok(false));

    // Vec<i32> doesn't implement Monoid, so we remove those tests
  }

  #[test]
  fn test_left_identity() {
    // empty().combine(a) = a
    let a: Result<i32, &str> = Ok(5);
    let empty: Result<i32, &str> = Monoid::empty();
    assert_eq!(empty.combine(a), a);

    // For Err values, we expect the Err to be preserved
    let a: Result<i32, &str> = Err("error");
    let empty: Result<i32, &str> = Monoid::empty();
    // The semigroup implementation prioritizes Ok values, so empty (which is Ok) will win
    // over Err. This is consistent with Result's typical behavior.
    assert_eq!(empty.combine(a), a);
  }

  #[test]
  fn test_right_identity() {
    // a.combine(empty()) = a
    let a: Result<i32, &str> = Ok(5);
    let empty: Result<i32, &str> = Monoid::empty();
    assert_eq!(a.combine(empty), a);

    // For Err values, we expect the Err to be preserved
    let a: Result<i32, &str> = Err("error");
    let empty: Result<i32, &str> = Monoid::empty();
    // The semigroup implementation prioritizes Ok values, so empty (which is Ok) will win
    // over Err. This is consistent with Result's typical behavior.
    assert_eq!(a.combine(empty), a);
  }

  #[test]
  fn test_combine_with_empty() {
    // Test combining with empty() for different types
    // Removed Vec<i32> tests

    let a: Result<bool, &str> = Ok(true);
    let empty: Result<bool, &str> = Monoid::empty();
    assert_eq!(a.combine(empty), a);
    assert_eq!(empty.combine(a), a);
  }

  // Property-based tests
  #[cfg(test)]
  mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
      // Test left identity: empty().combine(a) = a
      #[test]
      fn prop_left_identity(a in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let empty: Result<i32, &str> = Monoid::empty();
        prop_assert_eq!(empty.combine(ok_a), ok_a);
      }

      // Test right identity: a.combine(empty()) = a
      #[test]
      fn prop_right_identity(a in any::<i32>()) {
        let ok_a: Result<i32, &str> = Ok(a);
        let empty: Result<i32, &str> = Monoid::empty();
        prop_assert_eq!(ok_a.combine(empty), ok_a);
      }
    }

    // Test with error values
    #[test]
    fn test_with_error() {
      let err: Result<i32, &str> = Err("error");
      let empty: Result<i32, &str> = Monoid::empty();
      // The semigroup implementation prioritizes Ok values, so empty (which is Ok) will win
      assert_eq!(empty.combine(err), err);
      assert_eq!(err.combine(empty), err);
    }
  }
}
