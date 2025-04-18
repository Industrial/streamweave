// Implementation has been removed due to type system compatibility issues
// We will need to revisit the Bifunctor trait design

use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Bifunctor<A, B> for (A, B) {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = (C, D);

  fn bimap<C, D, F, G>(self, mut f: F, mut g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    (f(&self.0), g(&self.1))
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, B>
  where
    F: for<'a> FnMut(&'a A) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    (f(&self.0), self.1)
  }

  fn second<D, G>(self, mut g: G) -> Self::HigherSelf<A, D>
  where
    G: for<'b> FnMut(&'b B) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    (self.0, g(&self.1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_tuple_bifunctor_laws(a: i32, b: i32) {
      // Test identity law
      let id_a = |x: &i32| *x;
      let id_b = |x: &i32| *x;
      let tuple = (a, b);
      let result = tuple.bimap(id_a, id_b);
      assert_eq!(tuple, result);

      // Test composition law
      let f1 = |x: &i32| x.saturating_add(1);
      let f2 = |x: &i32| x.saturating_mul(2);
      let g1 = |x: &i32| x.saturating_sub(1);
      let g2 = |x: &i32| x.saturating_div(2);

      let f1_clone = f1;
      let f2_clone = f2;
      let g1_clone = g1;
      let g2_clone = g2;

      let result1 = tuple.bimap(f1, g1).bimap(f2, g2);
      let result2 = tuple.bimap(
        move |x| f2_clone(&f1_clone(x)),
        move |x| g2_clone(&g1_clone(x))
      );
      assert_eq!(result1, result2);

      // Test that first and second commute
      let f = |x: &i32| x.saturating_add(1);
      let g = |x: &i32| x.saturating_mul(2);

      let first_then_second = tuple.first(f).second(g);
      let second_then_first = tuple.second(g).first(f);

      assert_eq!(first_then_second, second_then_first);
    }
  }
}
