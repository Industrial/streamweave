use crate::traits::semigroup::Semigroup;
use std::time::Duration;

impl Semigroup for Duration {
  fn combine(self, other: Self) -> Self {
    self
      .checked_add(other)
      .unwrap_or_else(|| Duration::from_secs(u64::MAX))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_zero() {
    let zero = Duration::from_secs(0);
    let one_sec = Duration::from_secs(1);

    assert_eq!(zero.combine(zero), zero);
    assert_eq!(zero.combine(one_sec), one_sec);
    assert_eq!(one_sec.combine(zero), one_sec);
  }

  #[test]
  fn test_combine_normal_values() {
    let one_sec = Duration::from_secs(1);
    let two_sec = Duration::from_secs(2);
    let three_sec = Duration::from_secs(3);

    assert_eq!(one_sec.combine(two_sec), three_sec);
  }

  #[test]
  fn test_combine_with_millis() {
    let half_sec = Duration::from_millis(500);
    let one_sec = Duration::from_secs(1);
    let expected = Duration::from_millis(1500);

    assert_eq!(half_sec.combine(one_sec), expected);
  }

  #[test]
  fn test_overflow_handling() {
    let max = Duration::from_secs(u64::MAX);
    let one_sec = Duration::from_secs(1);

    // When overflow occurs, it should return u64::MAX
    assert_eq!(max.combine(one_sec), max);
  }

  #[test]
  fn test_associativity_manual() {
    let small = Duration::from_millis(100);
    let medium = Duration::from_secs(1);
    let large = Duration::from_secs(10);

    test_associativity(small, medium, large);
  }

  // Property-based test for durations (using reasonable limits)
  proptest! {
    // Generate durations from 0 to 100 seconds
    #[test]
    fn prop_associativity(
      a in 0u64..100,
      b in 0u64..100,
      c in 0u64..100
    ) {
      let dur_a = Duration::from_secs(a);
      let dur_b = Duration::from_secs(b);
      let dur_c = Duration::from_secs(c);

      let left = dur_a.combine(dur_b).combine(dur_c);
      let right = dur_a.combine(dur_b.combine(dur_c));

      prop_assert_eq!(left, right);
    }

    #[test]
    fn prop_addition_equivalence(a in 0u64..100, b in 0u64..100) {
      let dur_a = Duration::from_secs(a);
      let dur_b = Duration::from_secs(b);

      // Normal combine should be equivalent to addition when no overflow
      prop_assert_eq!(dur_a.combine(dur_b), dur_a + dur_b);
    }

    #[test]
    fn prop_overflow_handled(a in prop::num::u64::ANY, b in prop::num::u64::ANY) {
      let dur_a = Duration::from_secs(a);
      let dur_b = Duration::from_secs(b);

      // Result should always be valid, even with potential overflows
      let result = dur_a.combine(dur_b);

      // For any input, we should get a result (no panics)
      // If overflow, result should be the max value
      if let Some(sum) = dur_a.checked_add(dur_b) {
        prop_assert_eq!(result, sum);
      } else {
        prop_assert_eq!(result, Duration::from_secs(u64::MAX));
      }
    }
  }
}
