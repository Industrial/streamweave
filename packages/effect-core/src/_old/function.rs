//! Function type and implementations.
//!
//! A function type that implements various functional programming traits.

use super::category::Category;
use super::contravariant::Contravariant;
use super::profunctor::Profunctor;
use std::marker::PhantomData;
use std::sync::Arc;

/// A function type that implements various functional programming traits.
///
/// # Safety
///
/// This type requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement Send + Sync + 'static
/// - The type parameter B must implement Send + Sync + 'static
/// - The function f must implement Send + Sync + 'static
pub struct Function<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  f: Arc<dyn Fn(A) -> B + Send + Sync>,
  _phantom: PhantomData<(A, B)>,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Function<A, B> {
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

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Function<A, B> {
  fn clone(&self) -> Self {
    Self {
      f: self.f.clone(),
      _phantom: PhantomData,
    }
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Profunctor<A, B> for Function<A, B> {
  type HigherSelf<C: Send + Sync + 'static, D: Send + Sync + 'static> = Function<C, D>;

  fn dimap<C: Send + Sync + 'static, D: Send + Sync + 'static, F, G>(
    self,
    f: F,
    g: G,
  ) -> Self::HigherSelf<C, D>
  where
    F: Fn(C) -> A + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Function::new(move |x| g(self.apply(f(x))))
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Category<(), ()> for Function<A, B> {
  type Morphism<C: Send + Sync + 'static, D: Send + Sync + 'static> = Function<C, D>;

  fn id<C: Send + Sync + 'static>() -> Self::Morphism<C, C> {
    Function::new(|x| x)
  }

  fn compose<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
    g: Self::Morphism<D, E>,
  ) -> Self::Morphism<C, E> {
    Function::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + Send + Sync + 'static,
  {
    Function::new(f)
  }

  fn first<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(C, E), (D, E)> {
    Function::new(move |(x, y)| (f.apply(x), y))
  }

  fn second<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(E, C), (E, D)> {
    Function::new(move |(x, y)| (x, f.apply(y)))
  }
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Contravariant<A, B> for Function<A, B> {
  fn contramap<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<D, C>
  where
    F: Fn(D) -> C + Send + Sync + 'static,
  {
    Function::new(f)
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
    fn test_function_dimap(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];
      let function = Function::new(|x: i32| x + 1);
      let dimapped = function.dimap(f, g);
      assert_eq!(dimapped.apply(x), g(function.apply(f(x))));
    }

    #[test]
    fn test_function_lmap(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let function = Function::new(|x: i32| x + 1);
      let lmap = function.lmap(f);
      assert_eq!(lmap.apply(x), function.apply(f(x)));
    }

    #[test]
    fn test_function_rmap(
      x in any::<i32>(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let g = FUNCTIONS[g_idx];
      let function = Function::new(|x: i32| x + 1);
      let rmap = function.rmap(g);
      assert_eq!(rmap.apply(x), g(function.apply(x)));
    }

    #[test]
    fn test_function_category_id(
      x in any::<i32>()
    ) {
      let id = Function::<i32, i32>::id();
      assert_eq!(id.apply(x), x);
    }

    #[test]
    fn test_function_category_compose(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = Function::new(FUNCTIONS[f_idx]);
      let g = Function::new(FUNCTIONS[g_idx]);
      let composed = Function::compose(f, g);
      assert_eq!(composed.apply(x), g.apply(f.apply(x)));
    }

    #[test]
    fn test_function_contravariant(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let contramap = Function::<i32, i32>::contramap::<i32, i32, _>(f);
      assert_eq!(contramap.apply(x), f(x));
    }
  }
}
