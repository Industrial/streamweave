use proptest::prelude::*;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::Arc;

pub trait Category {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A>;
  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>;
}

pub struct Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  f: Arc<dyn Fn(A) -> B + Send + Sync + 'static>,
  _phantom: PhantomData<(A, B)>,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Morphism<A, B> {
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Morphism<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Self {
      f: Arc::new(f),
      _phantom: PhantomData,
    }
  }

  pub fn apply(&self, x: A) -> B {
    (self.f)(x)
  }
}

impl Category for Morphism<(), ()> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Morphism<A, B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Morphism::new(|x| x)
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Morphism::new(move |x| g.apply(f.apply(x)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  proptest! {
    #[test]
    fn test_category_composition(a: i32) {
      let f = Morphism::new(|x: i32| x.checked_add(1).unwrap_or(i32::MAX));
      let g = Morphism::new(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let composed = <Morphism<(), ()> as Category>::compose(f, g);
      let expected = a.checked_add(1)
        .and_then(|x| x.checked_mul(2))
        .unwrap_or(i32::MAX);
      assert_eq!(composed.apply(a), expected);
    }

    #[test]
    fn test_category_identity(a: i32) {
      let id = <Morphism<(), ()> as Category>::id::<i32>();
      assert_eq!(id.apply(a), a);
    }

    #[test]
    fn test_category_laws(a: i32) {
      // Test identity laws
      let f = Morphism::new(|x: i32| x.checked_mul(2).unwrap_or(i32::MAX));
      let id = <Morphism<(), ()> as Category>::id::<i32>();

      let f_id = <Morphism<(), ()> as Category>::compose(f.clone(), id.clone());
      let id_f = <Morphism<(), ()> as Category>::compose(id, f.clone());

      let expected = a.checked_mul(2).unwrap_or(i32::MAX);
      assert_eq!(f_id.apply(a), expected);
      assert_eq!(id_f.apply(a), expected);

      // Test associativity
      let g = Morphism::new(|x: i32| x.checked_add(1).unwrap_or(i32::MAX));
      let h = Morphism::new(|x: i32| x.checked_mul(3).unwrap_or(i32::MAX));

      let comp1 = <Morphism<(), ()> as Category>::compose(
        <Morphism<(), ()> as Category>::compose(f.clone(), g.clone()),
        h.clone()
      );
      let comp2 = <Morphism<(), ()> as Category>::compose(
        f,
        <Morphism<(), ()> as Category>::compose(g, h)
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
