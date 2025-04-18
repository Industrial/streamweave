use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Functor<T> for Option<T> {
  type HigherSelf<U: CloneableThreadSafe> = Option<U>;

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a T) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.map(|x| f(&x))
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
  fn test_identity_law_some() {
    // Choose a reasonable value for testing
    let x = 42i64;
    let option = Some(x);
    let id = |x: &i64| *x;
    // Use our Functor trait's map implementation
    let result = Functor::map(option, id);

    // Identity law: functor.map(id) == functor
    assert_eq!(result, Some(x));
  }

  #[test]
  fn test_identity_law_none() {
    let option: Option<i64> = None;
    let id = |x: &i64| *x;
    // Use our Functor trait's map implementation
    let result = Functor::map(option, id);

    // Identity law: functor.map(id) == functor
    assert_eq!(result, None);
  }

  proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_identity_law_some_prop(x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)) {
      let option = Some(x);
      let id = |x: &i64| *x;
      // Use our Functor trait's map implementation
      let result = Functor::map(option, id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(result, Some(x));
    }

    #[test]
    fn test_composition_law_some(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let option = Some(x);

      // Apply f then g using our Functor trait's map implementation
      let result1 = Functor::map(option, f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(option, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_composition_law_none(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];
      let option: Option<i64> = None;

      // Apply f then g using our Functor trait's map implementation
      let result1 = Functor::map(option, f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(option, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    // Implementation-specific test: handling of None values
    #[test]
    fn test_none_handling(f_idx in 0..INT_FUNCTIONS.len()) {
      let f = INT_FUNCTIONS[f_idx];
      let option: Option<i64> = None;

      // Mapping over None should preserve the None structure
      let result = Functor::map(option, f);
      prop_assert_eq!(result, None);
    }
  }
}
