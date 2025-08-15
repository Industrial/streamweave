use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;
use std::time::Duration;

/// A cloneable function wrapper for Duration
#[derive(Clone)]
pub struct DurationFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> DurationFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    DurationFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

impl Category<Duration, Duration> for Duration {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = DurationFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    DurationFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    DurationFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    DurationFn::new(move |x| f(&x))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    DurationFn::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    DurationFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for Duration
  fn double_duration(d: &Duration) -> Duration {
    d.saturating_mul(2)
  }

  fn add_second(d: &Duration) -> Duration {
    d.saturating_add(Duration::from_secs(1))
  }

  #[test]
  fn test_identity_law() {
    let d = Duration::from_secs(5);
    let id = <Duration as Category<Duration, Duration>>::id();

    assert_eq!(id.apply(d), d);
  }

  #[test]
  fn test_composition_law() {
    let d = Duration::from_secs(5);
    let f = <Duration as Category<Duration, Duration>>::arr(double_duration);
    let g = <Duration as Category<Duration, Duration>>::arr(add_second);

    let f_then_g = <Duration as Category<Duration, Duration>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(d));

    assert_eq!(f_then_g.apply(d), expected);
    assert_eq!(f_then_g.apply(Duration::from_secs(5)), Duration::from_secs(11)); // (5*2)+1 = 11
  }

  #[test]
  fn test_arr_law() {
    let d = Duration::from_secs(5);
    let f = |x: &Duration| x.saturating_mul(2);
    let arr_f = <Duration as Category<Duration, Duration>>::arr(f);

    let result = arr_f.apply(d);
    let expected = Duration::from_secs(10);

    assert_eq!(result, expected);
  }

  #[test]
  fn test_first() {
    let pair = (Duration::from_secs(5), "test");
    let f = <Duration as Category<Duration, Duration>>::arr(double_duration);
    let first_f = <Duration as Category<Duration, Duration>>::first(f);

    let result = first_f.apply(pair);
    assert_eq!(result, (Duration::from_secs(10), "test"));
  }

  #[test]
  fn test_second() {
    let pair = ("test", Duration::from_secs(5));
    let f = <Duration as Category<Duration, Duration>>::arr(double_duration);
    let second_f = <Duration as Category<Duration, Duration>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result, ("test", Duration::from_secs(10)));
  }

  proptest! {
    #[test]
    fn prop_identity_preserves_structure(
      d in any::<Duration>()
    ) {
      let id = <Duration as Category<Duration, Duration>>::id();
      let result = id.apply(d);

      assert_eq!(result, d);
    }

    #[test]
    fn prop_composition_preserves_structure(
      d in any::<Duration>()
    ) {
      let f = <Duration as Category<Duration, Duration>>::arr(double_duration);
      let g = <Duration as Category<Duration, Duration>>::arr(add_second);

      let f_then_g = <Duration as Category<Duration, Duration>>::compose(f, g);
      let result = f_then_g.apply(d);

      // Check that the transformation is correct
      let expected = d.saturating_mul(2).saturating_add(Duration::from_secs(1));
      assert_eq!(result, expected);
    }

    #[test]
    fn prop_arr_preserves_structure(
      d in any::<Duration>()
    ) {
      let f = |x: &Duration| x.saturating_mul(2);
      let arr_f = <Duration as Category<Duration, Duration>>::arr(f);

      let result = arr_f.apply(d);

      // Check that the transformation is correct
      assert_eq!(result, d.saturating_mul(2));
    }

    #[test]
    fn prop_first_preserves_structure(
      d in any::<Duration>(),
      c in any::<String>()
    ) {
      let f = <Duration as Category<Duration, Duration>>::arr(double_duration);
      let first_f = <Duration as Category<Duration, Duration>>::first(f);

      let input = (d, c.clone());
      let result = first_f.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, d.saturating_mul(2));
      assert_eq!(result.1, c);
    }

    #[test]
    fn prop_second_preserves_structure(
      c in any::<String>(),
      d in any::<Duration>()
    ) {
      let f = <Duration as Category<Duration, Duration>>::arr(double_duration);
      let second_f = <Duration as Category<Duration, Duration>>::second(f);

      let input = (c.clone(), d);
      let result = second_f.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, c);
      assert_eq!(result.1, d.saturating_mul(2));
    }
  }
} 