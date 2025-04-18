use crate::traits::bifunctor::Bifunctor;
use crate::types::either::Either;
use crate::types::threadsafe::CloneableThreadSafe;

impl<L: CloneableThreadSafe, R: CloneableThreadSafe> Bifunctor<L, R> for Either<L, R> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Either<C, D>;

  fn bimap<C, D, F, G>(self, mut f: F, mut g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a L) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b R) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    match self {
      Either::Left(a) => Either::Left(f(&a)),
      Either::Right(b) => Either::Right(g(&b)),
    }
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, R>
  where
    F: for<'a> FnMut(&'a L) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    match self {
      Either::Left(a) => Either::Left(f(&a)),
      Either::Right(b) => Either::Right(b),
    }
  }

  fn second<D, G>(self, mut g: G) -> Self::HigherSelf<L, D>
  where
    G: for<'b> FnMut(&'b R) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    match self {
      Either::Left(a) => Either::Left(a),
      Either::Right(b) => Either::Right(g(&b)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test the Either Bifunctor implementation
  proptest! {
    #[test]
    fn test_either_bifunctor_laws(a: i32, b: i32, left_or_right: bool) {
      let either = if left_or_right {
        Either::Left(a)
      } else {
        Either::Right(b)
      };

      // Test identity law
      let id_a = |x: &i32| *x;
      let id_b = |x: &i32| *x;
      let result = either.clone().bimap(id_a, id_b);
      assert_eq!(either, result);

      // Test composition law
      let f1 = |x: &i32| x.saturating_add(1);
      let f2 = |x: &i32| x.saturating_mul(2);
      let g1 = |x: &i32| x.saturating_sub(1);
      let g2 = |x: &i32| x.saturating_div(2);

      let f1_clone = f1;
      let f2_clone = f2;
      let g1_clone = g1;
      let g2_clone = g2;

      let result1 = either.clone().bimap(f1, g1).bimap(f2, g2);
      let result2 = either.clone().bimap(
        move |x| f2_clone(&f1_clone(x)),
        move |x| g2_clone(&g1_clone(x))
      );
      assert_eq!(result1, result2);

      // Test that first and second commute
      let f = |x: &i32| x.saturating_add(1);
      let g = |x: &i32| x.saturating_mul(2);

      let first_then_second = either.clone().first(f).second(g);
      let second_then_first = either.clone().second(g).first(f);

      assert_eq!(first_then_second, second_then_first);
    }
  }
}
