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
  use std::fmt::Debug;

  // Define test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Functions that may return the same value for certain inputs
  const SPECIAL_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| if *x < 0 { 0 } else { *x },           // ReLU-like function
    |x| *x % 10,                               // Modulo function
    |x| if *x % 2 == 0 { *x } else { *x + 1 }, // Make even
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

  #[test]
  fn test_map_with_string() {
    // Test with a different type (String)
    let option = Some("hello");
    let f = |s: &&str| s.to_uppercase();
    let result = Functor::map(option, f);
    assert_eq!(result, Some("HELLO".to_string()));
  }

  #[test]
  fn test_map_with_struct() {
    // Test with a custom struct
    #[derive(Debug, PartialEq, Clone)]
    struct Person {
      name: String,
      age: u32,
    }

    let person = Person {
      name: "Alice".to_string(),
      age: 30,
    };
    let option = Some(person);

    let f = |p: &Person| p.age;
    let result = Functor::map(option, f);
    assert_eq!(result, Some(30));
  }

  #[test]
  fn test_map_to_option() {
    // Test mapping to an Option type
    let option = Some(10);
    let f = |x: &i64| Some(*x * 2);
    let result = Functor::map(option, f);
    assert_eq!(result, Some(Some(20)));

    let option: Option<i64> = None;
    let result = Functor::map(option, f);
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
      let result1 = Functor::map(option.clone(), f);
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
      let result1 = Functor::map(option.clone(), f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(option, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    // Test with special functions that may return the same value for different inputs
    #[test]
    fn test_composition_with_special_functions(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..SPECIAL_FUNCTIONS.len(),
      g_idx in 0..SPECIAL_FUNCTIONS.len()
    ) {
      let f = SPECIAL_FUNCTIONS[f_idx];
      let g = SPECIAL_FUNCTIONS[g_idx];
      let option = Some(x);

      // Apply f then g
      let result1 = Functor::map(option.clone(), f);
      let result1 = Functor::map(result1, g);

      // Apply composition of f and g
      let composed = move |x: &i64| g(&f(x));
      let result2 = Functor::map(option, composed);

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }

    // Test with different integer values including edge cases
    #[test]
    fn test_with_edge_case_values(
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];

      // Edge case: i64::MAX
      let option = Some(i64::MAX);
      let result = Functor::map(option, f);
      prop_assert_eq!(result, Some(f(&i64::MAX)));

      // Edge case: i64::MIN
      let option = Some(i64::MIN);
      let result = Functor::map(option, f);
      prop_assert_eq!(result, Some(f(&i64::MIN)));

      // Edge case: 0
      let option = Some(0i64);
      let result = Functor::map(option, f);
      prop_assert_eq!(result, Some(f(&0)));
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

    // Test with nested Option
    #[test]
    fn test_nested_option(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 100),
      is_outer_some in any::<bool>(),
      is_inner_some in any::<bool>()
    ) {
      // Create nested Option based on boolean flags
      let inner_option = if is_inner_some { Some(x) } else { None };
      let nested_option = if is_outer_some { Some(inner_option) } else { None };

      // Map the outer Option - using saturating multiplication to avoid overflow
      let double = |opt: &Option<i64>| -> Option<i64> {
        opt.map(|x| x.saturating_mul(2))
      };

      let result = Functor::map(nested_option, double);

      // Expected result
      let expected = if is_outer_some {
        Some(if is_inner_some { Some(x.saturating_mul(2)) } else { None })
      } else {
        None
      };

      prop_assert_eq!(result, expected);
    }
  }
}
