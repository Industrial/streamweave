use crate::traits::monoid::Monoid;
use std::time::Duration;

impl Monoid for Duration {
  fn empty() -> Self {
    Duration::from_secs(0)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    assert_eq!(Duration::empty(), Duration::from_secs(0));
    assert_eq!(Duration::empty(), Duration::from_nanos(0));
  }

  #[test]
  fn test_empty_is_identity() {
    let zero = Duration::empty();
    let one_sec = Duration::from_secs(1);
    let one_min = Duration::from_secs(60);

    // Test that empty() is a left identity
    assert_eq!(zero.combine(one_sec), one_sec);
    assert_eq!(zero.combine(one_min), one_min);

    // Test that empty() is a right identity
    assert_eq!(one_sec.combine(zero), one_sec);
    assert_eq!(one_min.combine(zero), one_min);
  }

  #[test]
  fn test_mconcat_empty_iterator() {
    let durations: Vec<Duration> = vec![];
    assert_eq!(Duration::mconcat(durations), Duration::empty());
  }

  #[test]
  fn test_mconcat_with_values() {
    // Sum of durations
    let durations = vec![
      Duration::from_secs(1),
      Duration::from_secs(2),
      Duration::from_secs(3),
    ];

    assert_eq!(Duration::mconcat(durations), Duration::from_secs(6));
  }

  #[test]
  fn test_mconcat_with_mixed_values() {
    // Mixed values with different units
    let durations = vec![
      Duration::from_secs(1),
      Duration::from_millis(500),
      Duration::from_micros(1000),
    ];

    assert_eq!(Duration::mconcat(durations), Duration::from_millis(1501));
  }

  proptest! {
    #[test]
    fn prop_empty_is_left_identity(secs in 0u64..100) {
      let duration = Duration::from_secs(secs);
      prop_assert_eq!(Duration::empty().combine(duration), duration);
    }

    #[test]
    fn prop_empty_is_right_identity(secs in 0u64..100) {
      let duration = Duration::from_secs(secs);
      prop_assert_eq!(duration.combine(Duration::empty()), duration);
    }

    #[test]
    fn prop_mconcat_equivalent_to_fold(
      // Generate a vector of 0-10 durations from 0-10 seconds each
      vec in proptest::collection::vec(0u64..10, 0..10)
    ) {
      let durations: Vec<Duration> = vec.iter().map(|&s| Duration::from_secs(s)).collect();

      let mconcat_result = Duration::mconcat(durations.clone());
      let fold_result = durations.into_iter().fold(Duration::empty(), |acc, x| acc.combine(x));

      prop_assert_eq!(mconcat_result, fold_result);
    }
  }
}
