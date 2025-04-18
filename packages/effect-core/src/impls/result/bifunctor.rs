use crate::traits::bifunctor::Bifunctor;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Bifunctor<T, E> for Result<T, E> {
  type HigherSelf<C: CloneableThreadSafe, D: CloneableThreadSafe> = Result<C, D>;

  fn bimap<C, D, F, G>(self, mut f: F, mut g: G) -> Self::HigherSelf<C, D>
  where
    F: for<'a> FnMut(&'a T) -> C + CloneableThreadSafe,
    G: for<'b> FnMut(&'b E) -> D + CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    match self {
      Ok(a) => Ok(f(&a)),
      Err(b) => Err(g(&b)),
    }
  }

  fn first<C, F>(self, mut f: F) -> Self::HigherSelf<C, E>
  where
    F: for<'a> FnMut(&'a T) -> C + CloneableThreadSafe,
    C: CloneableThreadSafe,
  {
    match self {
      Ok(a) => Ok(f(&a)),
      Err(b) => Err(b),
    }
  }

  fn second<D, G>(self, mut g: G) -> Self::HigherSelf<T, D>
  where
    G: for<'b> FnMut(&'b E) -> D + CloneableThreadSafe,
    D: CloneableThreadSafe,
  {
    match self {
      Ok(a) => Ok(a),
      Err(b) => Err(g(&b)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_result_bifunctor_laws(
      a in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      e in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      success in any::<bool>()
    ) {
      let f1 = |x: &i32| x.saturating_add(1);
      let g1 = |x: &i32| x.saturating_mul(2);
      let f2 = |x: &i32| x.saturating_sub(3);
      let g2 = |x: &i32| x.saturating_div(2);

      let id_i32 = |x: &i32| *x;
      let id_err = |x: &i32| *x;

      let value: Result<i32, i32> = if success { Ok(a) } else { Err(e) };

      // Identity law: bimap(id, id) = id
      let id_mapped = value.clone().bimap(id_i32, id_err);
      assert_eq!(id_mapped, value);

      // Composition law:
      // bimap(f1, g1).bimap(f2, g2) = bimap(f2 ∘ f1, g2 ∘ g1)
      let f1_clone = f1;
      let f2_clone = f2;
      let g1_clone = g1;
      let g2_clone = g2;

      let composed1 = value.clone().bimap(f1, g1).bimap(f2, g2);
      let composed2 = value.clone().bimap(
        move |x| f2_clone(&f1_clone(x)),
        move |x| g2_clone(&g1_clone(x))
      );
      assert_eq!(composed1, composed2);

      // First/Second Commutativity law:
      // first(f).second(g) = second(g).first(f)
      let first_then_second = value.clone().first(f1).second(g1);
      let second_then_first = value.clone().second(g1).first(f1);
      assert_eq!(first_then_second, second_then_first);
    }
  }
}
