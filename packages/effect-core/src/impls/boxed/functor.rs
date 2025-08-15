use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Functor<T> for Box<T> {
  type HigherSelf<U: CloneableThreadSafe> = Box<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    Box::new(f(&*self))
  }

  fn map_owned<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    Box::new(f(*self))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  #[test]
  fn test_identity_law() {
    let x = 42i64;
    let boxed = Box::new(x);
    let id = |x: &i64| *x;
    let result = boxed.map(id);

    // Identity law: functor.map(id) == functor
    assert_eq!(*result, x);
  }

  proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_identity_law_prop(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      let boxed = Box::new(x);
      let id = |x: &i64| *x;
      let result = boxed.map(id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(*result, x);
    }

    #[test]
    fn test_composition_law(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create clones since map consumes self
      let boxed1 = Box::new(x);
      let boxed2 = Box::new(x);

      // Create captured versions of the functions to move into closures
      let f_captured = f;
      let g_captured = g;

      // Apply f then g
      let result1 = boxed1.map(f_captured).map(g_captured);

      // Apply composition of f and g
      let result2 = boxed2.map(move |x| {
        let intermediate = f_captured(x);
        g_captured(&intermediate)
      });

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(*result1, *result2);
    }

    // Implementation-specific test: single ownership semantics
    #[test]
    fn test_box_ownership_semantics(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let boxed = Box::new(x);

      // Box::map should move the original box
      let result = boxed.map(f);

      // The result should be a new Box
      prop_assert_eq!(*result, f(&x));

      // The original boxed value has been moved, so we can't use it anymore
      // This is a compile-time check, no need for runtime assertion
    }
  }
}
