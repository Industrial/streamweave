use super::category::Category;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait Bifunctor: Category {
  fn bimap<A, B, C, D, F, G>(f: F, g: G) -> Self::Morphism<(A, B), (C, D)>
  where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static;

  fn first<A, B, C, F>(f: F) -> Self::Morphism<(A, B), (C, B)>
  where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    C: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
  {
    Self::bimap(f, |x| x)
  }

  fn second<A, B, D, G>(g: G) -> Self::Morphism<(A, B), (A, D)>
  where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    D: Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Self::bimap(|x| x, g)
  }
}

#[cfg(test)]
mod tests {
  use super::super::category::Morphism;
  use super::*;
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn test_bifunctor_bimap(a: i32, b: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let bimap = <Morphism<(), ()> as Bifunctor>::bimap(f, g);
          let expected = (
              a.checked_mul(2).unwrap_or(i32::MAX),
              b.checked_add(1).unwrap_or(i32::MAX)
          );
          assert_eq!(bimap.apply((a, b)), expected);
      }

      #[test]
      fn test_bifunctor_first(a: i32, b: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let first = <Morphism<(), ()> as Bifunctor>::first(f);
          let expected = (a.checked_mul(2).unwrap_or(i32::MAX), b);
          assert_eq!(first.apply((a, b)), expected);
      }

      #[test]
      fn test_bifunctor_second(a: i32, b: i32) {
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let second = <Morphism<(), ()> as Bifunctor>::second(g);
          let expected = (a, b.checked_add(1).unwrap_or(i32::MAX));
          assert_eq!(second.apply((a, b)), expected);
      }

      #[test]
      fn test_bifunctor_laws(a: i32, b: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let h = |x: i32| x.checked_mul(3).unwrap_or(i32::MAX);
          let k = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);

          // Test first . second = second . first
          let first_then_second = <Morphism<(), ()> as Category>::compose(
              <Morphism<(), ()> as Bifunctor>::first(f),
              <Morphism<(), ()> as Bifunctor>::second(g)
          );

          let second_then_first = <Morphism<(), ()> as Category>::compose(
              <Morphism<(), ()> as Bifunctor>::second(g),
              <Morphism<(), ()> as Bifunctor>::first(f)
          );

          let input = (a, b);
          let result1 = first_then_second.apply(input.clone());
          let result2 = second_then_first.apply(input);
          assert_eq!(result1, result2);

          // Test bimap composition
          let bimap1 = <Morphism<(), ()> as Bifunctor>::bimap(f, g);
          let bimap2 = <Morphism<(), ()> as Bifunctor>::bimap(h, k);
          let composed = <Morphism<(), ()> as Category>::compose(bimap1, bimap2);

          let expected = (
              a.checked_mul(2)
                  .and_then(|x| x.checked_mul(3))
                  .unwrap_or(i32::MAX),
              b.checked_add(1)
                  .and_then(|x| x.checked_sub(1))
                  .unwrap_or(i32::MIN)
          );
          assert_eq!(composed.apply((a, b)), expected);
      }
  }
}
