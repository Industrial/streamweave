use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use crate::types::zipper::Zipper;
use std::sync::Arc;

impl<T: CloneableThreadSafe> Category<T, T> for Zipper<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> =
    Arc<dyn Fn(&A) -> B + Send + Sync + 'static>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    Arc::new(|a| a.clone())
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    let f = Arc::clone(&f);
    let g = Arc::clone(&g);
    Arc::new(move |a| g(&f(a)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    Arc::new(move |a| f(a))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Arc::new(move |&(ref a, ref c)| (f(a), c.clone()))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Arc::new(move |&(ref c, ref a)| (c.clone(), f(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_id() {
    let id = <Zipper<i32> as Category<i32, i32>>::id::<i32>();
    let x = 42;
    assert_eq!(id(&x), x);
  }

  #[test]
  fn test_compose() {
    let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_add(1));
    let g = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_mul(2));
    let h = <Zipper<i32> as Category<i32, i32>>::compose(f, g);

    assert_eq!(h(&5), 12); // (5 + 1) * 2 = 12
  }

  #[test]
  fn test_arr() {
    let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_mul(2));
    assert_eq!(f(&5), 10);
  }

  #[test]
  fn test_first() {
    let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_add(1));
    let first_f = <Zipper<i32> as Category<i32, i32>>::first::<i32, i32, String>(f);

    let input = (5, "test".to_string());
    let result = first_f(&input);

    assert_eq!(result.0, 6);
    assert_eq!(result.1, "test");
  }

  #[test]
  fn test_second() {
    let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_mul(3));
    let second_f = <Zipper<i32> as Category<i32, i32>>::second::<i32, i32, String>(f);

    let input = ("hello".to_string(), 7);
    let result = second_f(&input);

    assert_eq!(result.0, "hello");
    assert_eq!(result.1, 21);
  }

  proptest! {
      // Category law: id . f = f
      #[test]
      fn prop_left_identity(a in any::<i32>()) {
          let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_mul(2));
          let id = <Zipper<i32> as Category<i32, i32>>::id::<i32>();
          let composed = <Zipper<i32> as Category<i32, i32>>::compose(id, f.clone());

          prop_assert_eq!(f(&a), composed(&a));
      }

      // Category law: f . id = f
      #[test]
      fn prop_right_identity(a in any::<i32>()) {
          let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_mul(2));
          let id = <Zipper<i32> as Category<i32, i32>>::id::<i32>();
          let composed = <Zipper<i32> as Category<i32, i32>>::compose(f.clone(), id);

          prop_assert_eq!(f(&a), composed(&a));
      }

      // Category law: (f . g) . h = f . (g . h)
      #[test]
      fn prop_associativity(a in any::<i32>()) {
          let f = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_add(10));
          let g = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_mul(2));
          let h = <Zipper<i32> as Category<i32, i32>>::arr(|x: &i32| x.wrapping_sub(5));

          // (f . g) . h
          let fg = <Zipper<i32> as Category<i32, i32>>::compose(f.clone(), g.clone());
          let fgh = <Zipper<i32> as Category<i32, i32>>::compose(fg, h.clone());

          // f . (g . h)
          let gh = <Zipper<i32> as Category<i32, i32>>::compose(g, h);
          let fgh2 = <Zipper<i32> as Category<i32, i32>>::compose(f, gh);

          prop_assert_eq!(fgh(&a), fgh2(&a));
      }
  }
}
