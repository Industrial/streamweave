//! Pair type and implementations.
//!
//! A pair of functions that implement various functional programming traits.

use super::category::Category;
use super::contravariant::Contravariant;
use super::profunctor::Profunctor;
use std::marker::PhantomData;
use std::sync::Arc;

/// A pair of functions that implement various functional programming traits.
///
/// # Safety
///
/// This type requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement Send + Sync + 'static
/// - The type parameter B must implement Send + Sync + 'static
/// - The functions f and g must implement Send + Sync + 'static
pub struct Pair<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  f: Arc<dyn Fn(A) -> B + Send + Sync>,
  g: Arc<dyn Fn(B) -> A + Send + Sync>,
  _phantom: PhantomData<(A, B)>,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Pair<A, B> {
  pub fn new<F, G>(f: F, g: G) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
    G: Fn(B) -> A + Send + Sync + 'static,
  {
    Self {
      f: Arc::new(f),
      g: Arc::new(g),
      _phantom: PhantomData,
    }
  }

  pub fn apply(&self, x: A) -> B {
    (self.f)(x)
  }

  pub fn unapply(&self, y: B) -> A {
    (self.g)(y)
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Pair<A, B> {
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      g: self.g.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Profunctor<A, B> for Pair<A, B> {
  type HigherSelf<C: Send + Sync + 'static, D: Send + Sync + 'static> = Pair<C, D>;

  fn dimap<C: Send + Sync + 'static, D: Send + Sync + 'static, F, G>(
    self,
    f: F,
    g: G,
  ) -> Self::HigherSelf<C, D>
  where
    F: Fn(C) -> A + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Pair::new(move |x| g(self.apply(f(x))), move |y| f(self.unapply(g(y))))
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Category<(), ()> for Pair<A, B> {
  type Morphism<C: Send + Sync + 'static, D: Send + Sync + 'static> = Pair<C, D>;

  fn id<C: Send + Sync + 'static>() -> Self::Morphism<C, C> {
    Pair::new(|x| x, |x| x)
  }

  fn compose<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
    g: Self::Morphism<D, E>,
  ) -> Self::Morphism<C, E> {
    Pair::new(
      move |x| g.apply(f.apply(x)),
      move |x| f.unapply(g.unapply(x)),
    )
  }

  fn arr<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + Send + Sync + 'static,
  {
    Pair::new(f, |_| panic!("Cannot create inverse of arbitrary function"))
  }

  fn first<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(C, E), (D, E)> {
    Pair::new(
      move |(x, y)| (f.apply(x), y),
      move |(x, y)| (f.unapply(x), y),
    )
  }

  fn second<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(E, C), (E, D)> {
    Pair::new(
      move |(x, y)| (x, f.apply(y)),
      move |(x, y)| (x, f.unapply(y)),
    )
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Contravariant<A, B> for Pair<A, B> {
  fn contramap<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<D, C>
  where
    F: Fn(D) -> C + Send + Sync + 'static,
  {
    Pair::new(f, |_| panic!("Cannot create inverse of arbitrary function"))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions
  const FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  proptest! {
    #[test]
    fn test_pair_dimap(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];
      let pair = Pair::new(|x: i32| x + 1, |x: i32| x - 1);
      let dimapped = pair.dimap(f, g);
      assert_eq!(dimapped.apply(x), g(pair.apply(f(x))));
      assert_eq!(dimapped.unapply(g(pair.apply(f(x)))), f(x));
    }

    #[test]
    fn test_pair_category_id(
      x in any::<i32>()
    ) {
      let id = Pair::<i32, i32>::id();
      assert_eq!(id.apply(x), x);
      assert_eq!(id.unapply(x), x);
    }

    #[test]
    fn test_pair_category_compose(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = Pair::new(FUNCTIONS[f_idx], |x| x);
      let g = Pair::new(FUNCTIONS[g_idx], |x| x);
      let composed = Pair::compose(f, g);
      assert_eq!(composed.apply(x), g.apply(f.apply(x)));
    }

    #[test]
    fn test_pair_contravariant(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let contramap = Pair::<i32, i32>::contramap::<i32, i32, _>(f);
      assert_eq!(contramap.apply(x), f(x));
    }
  }
}
