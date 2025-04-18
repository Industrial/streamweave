// Implementation has been removed due to type system compatibility issues
// Option<T> can't properly implement Bifunctor as it only has one type parameter

use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;

// For Option<T>, we treat it as a container with a single type.
// We use () as a placeholder for the second type parameter.
impl<T: CloneableThreadSafe> Bifunctor<T, ()> for Option<T> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Option<C>;

  fn bimap<C, D, F, G>(self, mut f: F, _g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a T) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b ()) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    self.map(|x| f(&x))
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, ()>
  where
    F: for<'a> FnMut(&'a T) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    self.map(|x| f(&x))
  }

  fn second<D, G>(self, _g: G) -> Self::HigherSelf<T, D>
  where
    G: for<'b> FnMut(&'b ()) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test helper function for a specific Option value
  fn test_option_bifunctor_laws_for_value(opt: Option<i32>) {
    // Test identity law
    let id_a = |x: &i32| *x;
    let id_b = |_: &()| ();
    let result = opt.clone().bimap(id_a, id_b);
    assert_eq!(opt, result);

    // Test composition law
    let f1 = |x: &i32| x.saturating_add(1);
    let f2 = |x: &i32| x.saturating_mul(2);
    let g1 = |_: &()| ();
    let g2 = |_: &()| ();

    let f1_clone = f1;
    let f2_clone = f2;
    let g1_clone = g1;
    let g2_clone = g2;

    let result1 = opt.clone().bimap(f1, g1).bimap(f2, g2);
    let result2 = opt.clone().bimap(
      move |x| f2_clone(&f1_clone(x)),
      move |x| g2_clone(&g1_clone(x)),
    );
    assert_eq!(result1, result2);

    // Test that first and second commute
    let f = |x: &i32| x.saturating_add(1);
    let g = |_: &()| ();

    let first_then_second = opt.clone().first(f).second(g);
    let second_then_first = opt.clone().second(g).first(f);

    assert_eq!(first_then_second, second_then_first);
  }

  // Test specific variants explicitly
  #[test]
  fn test_option_none() {
    test_option_bifunctor_laws_for_value(None);
  }

  #[test]
  fn test_option_some() {
    // Test with various integers
    test_option_bifunctor_laws_for_value(Some(0));
    test_option_bifunctor_laws_for_value(Some(42));
    test_option_bifunctor_laws_for_value(Some(-123));
    test_option_bifunctor_laws_for_value(Some(i32::MAX));
    test_option_bifunctor_laws_for_value(Some(i32::MIN));
  }

  // Property-based tests for random values
  proptest! {
    #[test]
    fn test_option_bifunctor_laws_prop(maybe_val: Option<i32>) {
      test_option_bifunctor_laws_for_value(maybe_val);
    }
  }

  // Test additional transformations
  #[test]
  fn test_option_with_various_functions() {
    let opt = Some(10);

    // String conversion
    let string_fn = |x: &i32| x.to_string();
    let result = opt.clone().first(string_fn);
    assert_eq!(result, Some("10".to_string()));

    // Boolean transformation
    let bool_fn = |x: &i32| *x > 5;
    let result = opt.clone().first(bool_fn);
    assert_eq!(result, Some(true));

    // Option transformation (creating nested options)
    let option_fn = |x: &i32| Some(*x * 2);
    let result = opt.clone().first(option_fn);
    assert_eq!(result, Some(Some(20)));

    // None should propagate regardless of function
    let none: Option<i32> = None;
    let result = none.first(string_fn);
    assert_eq!(result, None);
  }
}
