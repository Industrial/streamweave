use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

/// A cloneable function wrapper for bool
#[derive(Clone)]
pub struct BoolFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> BoolFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    BoolFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

impl Category<bool, bool> for bool {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = BoolFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    BoolFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    BoolFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    BoolFn::new(move |x| f(&x))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    BoolFn::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    BoolFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for bools
  fn negate(b: &bool) -> bool {
    !b
  }

  fn to_string(b: &bool) -> String {
    b.to_string()
  }

  #[test]
  fn test_identity_law() {
    let b = true;
    let id = <bool as Category<bool, bool>>::id();

    assert_eq!(id.apply(b), b);
  }

  #[test]
  fn test_composition_law() {
    let b = true;
    let f = <bool as Category<bool, bool>>::arr(negate);
    let g = <bool as Category<bool, bool>>::arr(negate);

    let f_then_g = <bool as Category<bool, bool>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(b));

    assert_eq!(f_then_g.apply(b), expected);
    assert_eq!(f_then_g.apply(true), true); // !(!true) = true
  }

  #[test]
  fn test_arr_law() {
    let b = true;
    let f = |x: &bool| !x;
    let arr_f = <bool as Category<bool, bool>>::arr(f);

    let result = arr_f.apply(b);
    let expected = false;

    assert_eq!(result, expected);
  }

  #[test]
  fn test_first() {
    let pair = (true, 5);
    let f = <bool as Category<bool, bool>>::arr(negate);
    let first_f = <bool as Category<bool, bool>>::first(f);

    let result = first_f.apply(pair);
    assert_eq!(result, (false, 5));
  }

  #[test]
  fn test_second() {
    let pair = (5, true);
    let f = <bool as Category<bool, bool>>::arr(negate);
    let second_f = <bool as Category<bool, bool>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result, (5, false));
  }

  proptest! {
    #[test]
    fn prop_identity_preserves_structure(
      b in any::<bool>()
    ) {
      let id = <bool as Category<bool, bool>>::id();
      let result = id.apply(b);

      assert_eq!(result, b);
    }

    #[test]
    fn prop_composition_preserves_structure(
      b in any::<bool>()
    ) {
      let f = <bool as Category<bool, bool>>::arr(negate);
      let g = <bool as Category<bool, bool>>::arr(negate);

      let f_then_g = <bool as Category<bool, bool>>::compose(f, g);
      let result = f_then_g.apply(b);

      // Check that the transformation is correct (double negation)
      assert_eq!(result, b);
    }

    #[test]
    fn prop_arr_preserves_structure(
      b in any::<bool>()
    ) {
      let f = |x: &bool| !x;
      let arr_f = <bool as Category<bool, bool>>::arr(f);

      let result = arr_f.apply(b);

      // Check that the transformation is correct
      assert_eq!(result, !b);
    }

    #[test]
    fn prop_first_preserves_structure(
      b in any::<bool>(),
      c in any::<i32>()
    ) {
      let f = <bool as Category<bool, bool>>::arr(negate);
      let first_f = <bool as Category<bool, bool>>::first(f);

      let input = (b, c);
      let result = first_f.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, !b);
      assert_eq!(result.1, c);
    }

    #[test]
    fn prop_second_preserves_structure(
      c in any::<i32>(),
      b in any::<bool>()
    ) {
      let f = <bool as Category<bool, bool>>::arr(negate);
      let second_f = <bool as Category<bool, bool>>::second(f);

      let input = (c, b);
      let result = second_f.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, c);
      assert_eq!(result.1, !b);
    }
  }
}
