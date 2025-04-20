use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Bifunctor<A, B> for Arc<(A, B)> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Arc<(C, D)>;

  fn bimap<C, D, F, G>(self, mut f: F, mut g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    // Get a reference to the tuple, apply the bifunctor, then wrap in a new Arc
    let (a, b) = &*self;
    Arc::new((f(a), g(b)))
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, B>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    // Get a reference to the tuple, apply first, then wrap in a new Arc
    let (a, b) = &*self;
    Arc::new((f(a), b.clone()))
  }

  fn second<D, G>(self, mut g: G) -> Self::HigherSelf<A, D>
  where
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    // Get a reference to the tuple, apply second, then wrap in a new Arc
    let (a, b) = &*self;
    Arc::new((a.clone(), g(b)))
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
  fn test_arc_bifunctor_simple() {
    let arc_tuple = Arc::new((5, 10));

    // Test bimap
    let result = arc_tuple.bimap(double, increment);
    assert_eq!(*result, (10, 11));
  }

  #[test]
  fn test_arc_first_second() {
    let arc_tuple = Arc::new((5, 10));

    // Test first
    let result = arc_tuple.clone().first(double);
    assert_eq!(*result, (10, 10));

    // For second, we use the cloned arc_tuple
    let result = arc_tuple.second(increment);
    assert_eq!(*result, (5, 11));
  }

  #[test]
  fn test_arc_bimap_with_different_types() {
    let arc_tuple = Arc::new(("hello", 5));

    // String -> Length, Number -> Double
    let result = arc_tuple.bimap(len, double);
    assert_eq!(*result, (5, 10));
  }

  #[test]
  fn test_arc_first_second_different_types() {
    let arc_tuple = Arc::new(("hello", 5));

    // Test first with string -> length
    let result = arc_tuple.clone().first(len);
    assert_eq!(*result, (5, 5));

    // Test second with the cloned arc_tuple
    let result = arc_tuple.second(double);
    assert_eq!(*result, ("hello", 10));
  }

  // Test that first(f) . second(g) = second(g) . first(f)
  #[test]
  fn test_first_second_commutativity() {
    let arc_tuple = Arc::new((5, 10));

    // first(double) then second(increment)
    let first_then_second = arc_tuple.clone().first(double).second(increment);

    // second(increment) then first(double)
    let second_then_first = arc_tuple.second(increment).first(double);

    // Results should be the same
    assert_eq!(*first_then_second, *second_then_first);
  }

  // Property-based tests
  proptest! {
    #[test]
    fn prop_arc_bimap_identity(a in -1000..1000i32, b in -1000..1000i32) {
      let arc_tuple = Arc::new((a, b));

      // Identity functions
      let id_a = |x: &i32| *x;
      let id_b = |x: &i32| *x;

      // bimap(id, id) should be equivalent to id
      let result = arc_tuple.bimap(id_a, id_b);
      prop_assert_eq!(*result, (a, b));
    }

    #[test]
    fn prop_arc_first_second_commutativity(
      a in -1000..1000i32,
      b in -1000..1000i32,
      f_choice in 0..3u8,
      g_choice in 0..3u8
    ) {
      let arc_tuple = Arc::new((a, b));

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
      let first_then_second = arc_tuple.clone().first(f).second(g);

      // second(g) then first(f)
      let second_then_first = arc_tuple.clone().second(g).first(f);

      // Results should be the same
      prop_assert_eq!(*first_then_second, *second_then_first);
    }

    #[test]
    fn prop_arc_first_second_derivation(
      a in -1000..1000i32,
      b in -1000..1000i32,
      f_choice in 0..3u8
    ) {
      let arc_tuple = Arc::new((a, b));

      // Select function based on choice
      let f = match f_choice {
        0 => double,
        1 => increment,
        _ => square,
      };

      // Test that first is equivalent to bimap(f, id)
      let first_result = arc_tuple.clone().first(f);
      let id_b = |x: &i32| *x;
      let bimap_result = arc_tuple.clone().bimap(f, id_b);

      prop_assert_eq!(*first_result, *bimap_result);

      // Test that second is equivalent to bimap(id, g)
      let second_result = arc_tuple.clone().second(f);
      let id_a = |x: &i32| *x;
      let bimap_result = arc_tuple.bimap(id_a, f);

      prop_assert_eq!(*second_result, *bimap_result);
    }
  }
}
