// Implementation has been removed due to type system compatibility issues
// Vec<T> can't properly implement Bifunctor as it only has one type parameter

use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;

// For Vec<T>, we treat it as a container with a single type.
// We use () as a placeholder for the second type parameter.
impl<T: CloneableThreadSafe> Bifunctor<T, ()> for Vec<T> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Vec<C>;

  fn bimap<C, D, F, G>(self, mut f: F, _g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a T) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b ()) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    self.into_iter().map(|x| f(&x)).collect()
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, ()>
  where
    F: for<'a> FnMut(&'a T) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    self.into_iter().map(|x| f(&x)).collect()
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

  // Helper function to test Bifunctor laws for a specific vector
  fn test_vec_bifunctor_laws_for_value(xs: Vec<i32>) {
    // Test identity law
    let id_a = |x: &i32| *x;
    let id_b = |_: &()| ();
    let result = xs.clone().bimap(id_a, id_b);
    assert_eq!(xs, result);

    // Test composition law
    let f1 = |x: &i32| x.saturating_add(1);
    let f2 = |x: &i32| x.saturating_mul(2);
    let g1 = |_: &()| ();
    let g2 = |_: &()| ();

    let f1_clone = f1;
    let f2_clone = f2;
    let g1_clone = g1;
    let g2_clone = g2;

    let result1 = xs.clone().bimap(f1, g1).bimap(f2, g2);
    let result2 = xs.clone().bimap(
      move |x| f2_clone(&f1_clone(x)),
      move |x| g2_clone(&g1_clone(x)),
    );
    assert_eq!(result1, result2);

    // Test that first and second commute
    let f = |x: &i32| x.saturating_add(1);
    let g = |_: &()| ();

    let first_then_second = xs.clone().first(f).second(g);
    let second_then_first = xs.clone().second(g).first(f);

    assert_eq!(first_then_second, second_then_first);
  }

  // Test empty vectors explicitly
  #[test]
  fn test_empty_vec() {
    test_vec_bifunctor_laws_for_value(vec![]);
  }

  // Test single-element vectors
  #[test]
  fn test_singleton_vec() {
    test_vec_bifunctor_laws_for_value(vec![0]);
    test_vec_bifunctor_laws_for_value(vec![42]);
    test_vec_bifunctor_laws_for_value(vec![-123]);
    test_vec_bifunctor_laws_for_value(vec![i32::MAX]);
    test_vec_bifunctor_laws_for_value(vec![i32::MIN]);
  }

  // Test multi-element vectors with known values
  #[test]
  fn test_multi_element_vec() {
    test_vec_bifunctor_laws_for_value(vec![1, 2, 3, 4, 5]);
    test_vec_bifunctor_laws_for_value(vec![-10, 0, 10, 20]);
    test_vec_bifunctor_laws_for_value(vec![i32::MIN, -1, 0, 1, i32::MAX]);
  }

  // Property-based tests for random values
  proptest! {
    #[test]
    fn test_vec_bifunctor_laws_prop(xs in prop::collection::vec(any::<i32>(), 0..10)) {
      test_vec_bifunctor_laws_for_value(xs);
    }
  }

  // Test with various function types
  #[test]
  fn test_vec_with_various_functions() {
    let xs = vec![1, 2, 3, 4, 5];

    // String conversion
    let string_fn = |x: &i32| x.to_string();
    let result = xs.clone().first(string_fn);
    assert_eq!(
      result,
      vec![
        "1".to_string(),
        "2".to_string(),
        "3".to_string(),
        "4".to_string(),
        "5".to_string()
      ]
    );

    // Boolean transformation
    let bool_fn = |x: &i32| *x > 3;
    let result = xs.clone().first(bool_fn);
    assert_eq!(result, vec![false, false, false, true, true]);

    // Option transformation
    let option_fn = |x: &i32| if *x % 2 == 0 { Some(*x) } else { None };
    let result = xs.clone().first(option_fn);
    assert_eq!(result, vec![None, Some(2), None, Some(4), None]);

    // Empty vec should remain empty regardless of function
    let empty: Vec<i32> = vec![];
    let result = empty.first(string_fn);
    assert_eq!(result, Vec::<String>::new());
  }

  // Test stateful mapping functions
  #[test]
  fn test_stateful_functions() {
    let xs = vec![1, 2, 3, 4, 5];

    // Function that keeps track of how many elements it has seen
    let mut counter = 0;
    let stateful_fn = move |x: &i32| {
      counter += 1;
      *x * counter
    };

    let result = xs.first(stateful_fn);
    assert_eq!(result, vec![1, 4, 9, 16, 25]);
  }
}
