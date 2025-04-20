use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Bifunctor<A, B> for Box<(A, B)> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Box<(C, D)>;

  fn bimap<C, D, F, G>(self, mut f: F, mut g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    // Unbox the tuple, apply the bifunctor, then rebox
    let (a, b) = *self;
    Box::new((f(&a), g(&b)))
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, B>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    // Unbox the tuple, apply first, then rebox
    let (a, b) = *self;
    Box::new((f(&a), b))
  }

  fn second<D, G>(self, mut g: G) -> Self::HigherSelf<A, D>
  where
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    // Unbox the tuple, apply second, then rebox
    let (a, b) = *self;
    Box::new((a, g(&b)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for integers
  fn double(x: &i32) -> i32 {
    x * 2
  }

  fn increment(x: &i32) -> i32 {
    x + 1
  }

  fn square(x: &i32) -> i32 {
    x * x
  }

  // Test functions for strings
  fn len(s: &&str) -> usize {
    s.len()
  }

  #[test]
  fn test_boxed_bifunctor_simple() {
    let boxed_tuple = Box::new((5, 10));

    // Test bimap
    let result = boxed_tuple.bimap(double, increment);
    assert_eq!(*result, (10, 11));
  }

  #[test]
  fn test_boxed_first_second() {
    let boxed_tuple = Box::new((5, 10));

    // Test first
    let result = boxed_tuple.first(double);
    assert_eq!(*result, (10, 10));

    // For second, we need a new boxed tuple
    let boxed_tuple = Box::new((5, 10));
    let result = boxed_tuple.second(increment);
    assert_eq!(*result, (5, 11));
  }

  #[test]
  fn test_boxed_bimap_with_different_types() {
    let boxed_tuple = Box::new(("hello", 5));

    // String -> Length, Number -> Double
    let result = boxed_tuple.bimap(len, double);
    assert_eq!(*result, (5, 10));
  }

  #[test]
  fn test_boxed_first_second_different_types() {
    let boxed_tuple = Box::new(("hello", 5));

    // Test first with string -> length
    let result = boxed_tuple.first(len);
    assert_eq!(*result, (5, 5));

    // Test second with a new boxed tuple
    let boxed_tuple = Box::new(("hello", 5));
    let result = boxed_tuple.second(double);
    assert_eq!(*result, ("hello", 10));
  }

  // Test that first(f) . second(g) = second(g) . first(f)
  #[test]
  fn test_first_second_commutativity() {
    let boxed_tuple = Box::new((5, 10));

    // first(double) then second(increment)
    let first_then_second = boxed_tuple.first(double).second(increment);

    // Need a new boxed tuple for the second test
    let boxed_tuple = Box::new((5, 10));

    // second(increment) then first(double)
    let second_then_first = boxed_tuple.second(increment).first(double);

    // Results should be the same
    assert_eq!(*first_then_second, *second_then_first);
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_boxed_bimap_identity(a in -1000..1000i32, b in -1000..1000i32) {
      let boxed_tuple = Box::new((a, b));

      // Identity functions
      let id_a = |x: &i32| *x;
      let id_b = |x: &i32| *x;

      // bimap(id, id) should be equivalent to id
      let result = boxed_tuple.bimap(id_a, id_b);
      prop_assert_eq!(*result, (a, b));
    }

    #[test]
    fn prop_boxed_first_second_commutativity(
      a in -1000..1000i32,
      b in -1000..1000i32,
      f_choice in 0..3u8,
      g_choice in 0..3u8
    ) {
      let boxed_tuple = Box::new((a, b));

      // Select functions based on choices
      let f = match f_choice {
        0 => double,
        1 => increment,
        _ => square,
      };

      let g = match g_choice {
        0 => double,
        1 => increment,
        _ => square,
      };

      // first(f) then second(g)
      let first_then_second = boxed_tuple.clone().first(f).second(g);

      // second(g) then first(f)
      let second_then_first = boxed_tuple.clone().second(g).first(f);

      // Results should be the same
      prop_assert_eq!(*first_then_second, *second_then_first);
    }

    #[test]
    fn prop_boxed_first_second_derivation(
      a in -1000..1000i32,
      b in -1000..1000i32,
      f_choice in 0..3u8
    ) {
      let boxed_tuple = Box::new((a, b));

      // Select function based on choice
      let f = match f_choice {
        0 => double,
        1 => increment,
        _ => square,
      };

      // Test that first is equivalent to bimap(f, id)
      let first_result = boxed_tuple.clone().first(f);
      let id_b = |x: &i32| *x;
      let bimap_result = boxed_tuple.clone().bimap(f, id_b);

      prop_assert_eq!(*first_result, *bimap_result);

      // Test that second is equivalent to bimap(id, g)
      let second_result = boxed_tuple.clone().second(f);
      let id_a = |x: &i32| *x;
      let bimap_result = boxed_tuple.clone().bimap(id_a, f);

      prop_assert_eq!(*second_result, *bimap_result);
    }
  }
}
