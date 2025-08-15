use super::category::DurationFn;
use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use std::time::Duration;

impl Arrow<Duration, Duration> for Duration {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    DurationFn::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<Duration, Duration>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(Duration, C), (Duration, D)> {
    DurationFn::new(move |(d, c)| (f.apply(d), g.apply(c)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<Duration, Duration>,
    g: Self::Morphism<Duration, C>,
  ) -> Self::Morphism<Duration, (Duration, C)> {
    DurationFn::new(move |d: Duration| (f.apply(d.clone()), g.apply(d)))
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use proptest::prelude::*;
  use std::time::Duration;

  #[test]
  fn test_arrow_creation() {
    let f = Duration::arrow(|x: Duration| x.saturating_mul(2));
    let input = Duration::from_secs(5);
    let result = f.apply(input);
    assert_eq!(result, Duration::from_secs(10));
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = Duration::from_secs(5);

    let f_arrow = Duration::arrow(|x: Duration| x.saturating_mul(2));
    let f_arr = Duration::arr(|x: &Duration| x.saturating_mul(2));

    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);

    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = Duration::arrow(|x: Duration| x.saturating_mul(2));
    let g = Duration::arrow(|x: i32| x + 1);

    let split_fn = Duration::split::<i32, i32, i32, i32>(f, g);
    let input = (Duration::from_secs(5), 10);

    let result = split_fn.apply(input);
    assert_eq!(result, (Duration::from_secs(10), 11));
  }

  #[test]
  fn test_fanout() {
    let f = Duration::arrow(|x: Duration| x.saturating_mul(2));
    let g = Duration::arrow(|x: Duration| x.as_secs());

    let fanout_fn = Duration::fanout(f, g);
    let input = Duration::from_secs(5);

    let result = fanout_fn.apply(input);
    assert_eq!(result, (Duration::from_secs(10), 5));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      d in any::<Duration>(),
      y in any::<i32>()
    ) {
      let f = Duration::arrow(|x: Duration| x.saturating_mul(2));
      let g = Duration::arrow(|x: i32| x.saturating_add(1));

      let split_fn = Duration::split::<i32, i32, i32, i32>(f, g);
      let input = (d, y);

      let result = split_fn.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, d.saturating_mul(2));
      assert_eq!(result.1, y.saturating_add(1));
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      d in any::<Duration>()
    ) {
      let f = Duration::arrow(|x: Duration| x.saturating_mul(2));
      let g = Duration::arrow(|x: Duration| x.as_secs());

      let fanout_fn = Duration::fanout(f, g);
      let input = d;

      let result = fanout_fn.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, d.saturating_mul(2));
      assert_eq!(result.1, d.as_secs());
    }
  }
}
