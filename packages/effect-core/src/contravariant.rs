use super::category::Category;
use std::marker::PhantomData;
use std::sync::Arc;

pub trait Contravariant: Category {
  fn contramap<A, B, F>(f: F) -> Self::Morphism<B, A>
  where
    A: Send + Sync + 'static,
    B: Send + Sync + 'static,
    F: Fn(B) -> A + Send + Sync + 'static;
}

#[cfg(test)]
mod tests {
  use super::super::category::Morphism;
  use super::*;
  use proptest::prelude::*;

  proptest! {
      #[test]
      fn test_contravariant_contramap(a: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let contramap = <Morphism<(), ()> as Contravariant>::contramap::<i32, i32, _>(f);
          let composed = <Morphism<(), ()> as Category>::compose(contramap, Morphism::new(g));
          let expected = a.checked_mul(2)
              .and_then(|x| x.checked_add(1))
              .unwrap_or(i32::MAX);
          assert_eq!(composed.apply(a), expected);
      }

      #[test]
      fn test_contravariant_laws(a: i32) {
          let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
          let g = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
          let h = |x: i32| x.checked_mul(3).unwrap_or(i32::MAX);

          // Test identity law
          let id = <Morphism<(), ()> as Category>::id::<i32>();
          let contramap_id = <Morphism<(), ()> as Contravariant>::contramap::<i32, i32, _>(|x| x);
          let composed_id = <Morphism<(), ()> as Category>::compose(contramap_id, id);
          assert_eq!(composed_id.apply(a), a);

          // Test composition law
          let contramap_f = <Morphism<(), ()> as Contravariant>::contramap::<i32, i32, _>(f);
          let contramap_g = <Morphism<(), ()> as Contravariant>::contramap::<i32, i32, _>(g);
          let contramap_h = <Morphism<(), ()> as Contravariant>::contramap::<i32, i32, _>(h);

          let comp1 = <Morphism<(), ()> as Category>::compose(
              <Morphism<(), ()> as Category>::compose(contramap_f.clone(), contramap_g.clone()),
              contramap_h.clone()
          );

          let comp2 = <Morphism<(), ()> as Category>::compose(
              contramap_f,
              <Morphism<(), ()> as Category>::compose(contramap_g, contramap_h)
          );

          let expected = a.checked_mul(2)
              .and_then(|x| x.checked_add(1))
              .and_then(|x| x.checked_mul(3))
              .unwrap_or(i32::MAX);
          assert_eq!(comp1.apply(a), expected);
          assert_eq!(comp2.apply(a), expected);
      }
  }
}
