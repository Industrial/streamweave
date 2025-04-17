//! Core Effect type implementation.
//!
//! This module provides the `Effect` type, which represents a computation that
//! may produce a value of type `T` or fail with an error of type `E`. The
//! computation is performed asynchronously, and the result is wrapped in a
//! `Result` type.

use std::{future::Future, pin::Pin};

use crate::{
  Alternative, Applicative, Bifunctor, Category, Comonad, Filterable, Functor, Monad, Natural,
};

/// A type that represents an asynchronous computation that can yield a value of
/// type `T` or fail with an error of type `E`.
///
/// # Type Parameters
///
/// - T: The type of the successful result
/// - E: The type of the error
pub struct Effect<T, E> {
  inner: Pin<Box<dyn Future<Output = Result<T, E>> + Send + Sync>>,
}

impl<T, E> Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
{
  /// Creates a new Effect from a future
  pub fn new<F>(future: F) -> Self
  where
    F: Future<Output = Result<T, E>> + Send + Sync + 'static,
  {
    Self {
      inner: Box::pin(future),
    }
  }

  /// Creates a new Effect that always succeeds with the given value
  pub fn pure(value: T) -> Self
  where
    T: Send + Sync + 'static,
  {
    Self::new(std::future::ready(Ok(value)))
  }

  /// Creates a new Effect that always fails with the given error
  pub fn error(error: E) -> Self
  where
    E: Send + Sync + 'static,
  {
    Self::new(std::future::ready(Err(error)))
  }

  /// Runs the Effect and returns its result
  pub async fn run(&mut self) -> Result<T, E> {
    self.inner.as_mut().await
  }

  /// Chains a function that returns an Effect after this one
  pub fn and_then<U, F>(self, f: F) -> Effect<U, E>
  where
    F: FnOnce(T) -> Effect<U, E> + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    let future = async move {
      let mut self_ = self;
      let value = self_.run().await?;
      f(value).run().await
    };
    Effect::new(future)
  }

  /// Handles an error by converting it into a new Effect
  pub fn recover_with<F>(self, f: F) -> Self
  where
    F: FnOnce(E) -> Self + Send + Sync + 'static,
  {
    let future = async move {
      let mut self_ = self;
      match self_.run().await {
        Ok(value) => Ok(value),
        Err(e) => f(e).run().await,
      }
    };
    Effect::new(future)
  }
}

// Implement Functor with Send + Sync + 'static bounds
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Functor<T> for Effect<T, E> {
  type HigherSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn map<U: Send + Sync + 'static, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut self_ = self;
      let value = self_.run().await?;
      Ok(f(value))
    })
  }
}

// Implement Applicative
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Applicative<T> for Effect<T, E> {
  type ApplicativeSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn pure<U: Send + Sync + 'static>(value: U) -> Self::ApplicativeSelf<U> {
    Effect::new(async move { Ok(value) })
  }

  fn ap<U: Send + Sync + 'static, F>(self, f: Effect<F, E>) -> Effect<U, E>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut self_ = self;
      let mut f_ = f;
      let value = self_.run().await?;
      let mut func = f_.run().await?;
      Ok(func(value))
    })
  }
}

// Implement Monad
impl<T: Send + Sync + 'static, E: Send + Sync + 'static> Monad<T> for Effect<T, E> {
  type MonadSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn bind<U: Send + Sync + 'static, F>(self, mut f: F) -> Self::MonadSelf<U>
  where
    F: FnMut(T) -> Self::MonadSelf<U> + Send + Sync + 'static,
  {
    Effect::new(async move {
      let mut self_ = self;
      let value = self_.run().await?;
      f(value).run().await
    })
  }
}

// Implement Category<A, B>
impl<T, E, A, B> Category<A, B> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
  A: Send + Sync + 'static,
  B: Send + Sync + 'static,
{
  type Morphism<C: Send + Sync + 'static, D: Send + Sync + 'static> = Effect<D, E>;

  fn id<C: Send + Sync + 'static>() -> Self::Morphism<C, C> {
    Effect::new(async move { Ok(unsafe { std::mem::zeroed::<C>() }) })
  }

  fn compose<C: Send + Sync + 'static, D: Send + Sync + 'static, F: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
    g: Self::Morphism<D, F>,
  ) -> Self::Morphism<C, F> {
    Effect::new(async move {
      let mut f_ = f;
      let mut g_ = g;
      f_.run().await?;
      g_.run().await
    })
  }

  fn arr<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + Send + Sync + 'static,
  {
    Effect::new(async move { Ok(f(unsafe { std::mem::zeroed::<C>() })) })
  }

  fn first<C: Send + Sync + 'static, D: Send + Sync + 'static, F: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(C, F), (D, F)> {
    Effect::new(async move {
      let mut f_ = f;
      let value = f_.run().await?;
      Ok((value, unsafe { std::mem::zeroed::<F>() }))
    })
  }

  fn second<C: Send + Sync + 'static, D: Send + Sync + 'static, F: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(F, C), (F, D)> {
    Effect::new(async move {
      let mut f_ = f;
      let value = f_.run().await?;
      Ok((unsafe { std::mem::zeroed::<F>() }, value))
    })
  }
}

// Implement Bifunctor
impl<T, E, A, B> Bifunctor<A, B> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static,
  A: Send + Sync + 'static,
  B: Send + Sync + 'static,
{
  fn bimap<C, D, F, G>(f: F, g: G) -> Self::Morphism<(A, B), (C, D)>
  where
    C: Send + Sync + 'static,
    D: Send + Sync + 'static,
    F: Fn(A) -> C + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    Effect::new(async move {
      Ok((
        f(unsafe { std::mem::zeroed::<A>() }),
        g(unsafe { std::mem::zeroed::<B>() }),
      ))
    })
  }
}

// Implement Filterable
impl<T, E> Filterable<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static + From<&'static str>,
{
  type HigherSelf<U: Send + Sync + 'static> = Effect<U, E>;

  fn filter<F>(self, f: F) -> Self
  where
    F: FnOnce(&T) -> bool + Send + Sync + 'static,
  {
    let future = async move {
      let mut self_ = self;
      let value = self_.run().await?;
      if f(&value) {
        Ok(value)
      } else {
        Err(E::from("Filtered out"))
      }
    };
    Effect::new(future)
  }
}

// Implement Alternative
impl<T, E> Alternative<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static + Default,
{
  fn empty() -> Self {
    Effect::error(E::default())
  }

  fn alt(self, mut other: Self) -> Self {
    let future = async move {
      let mut self_ = self;
      match self_.run().await {
        Ok(value) => Ok(value),
        Err(_) => other.run().await,
      }
    };
    Effect::new(future)
  }
}

// Implement Comonad
impl<T, E> Comonad<T> for Effect<T, E>
where
  T: Send + Sync + 'static,
  E: Send + Sync + 'static + std::fmt::Debug,
{
  fn extract(self) -> T {
    // Note: This is a blocking operation
    futures::executor::block_on(async {
      let mut self_ = self;
      self_.run().await.unwrap()
    })
  }

  fn extend<U, F>(self, mut f: F) -> Effect<U, E>
  where
    F: FnMut(T) -> U + Send + Sync + 'static,
    U: Send + Sync + 'static,
  {
    let future = async move {
      let mut self_ = self;
      let value = self_.run().await?;
      Ok(f(value))
    };
    Effect::new(future)
  }
}

// Implement Natural for Option -> Effect
impl<E> Natural<Option<()>, Effect<(), E>> for Effect<(), E>
where
  E: Send + Sync + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Option<T>) -> Effect<T, E> {
    match fa {
      Some(value) => Effect::new(async move { Ok(value) }),
      None => Effect::new(async move { Err(unsafe { std::mem::zeroed::<E>() }) }),
    }
  }
}

// Implement Natural for Result -> Effect
impl<E> Natural<Result<(), E>, Effect<(), E>> for Effect<(), E>
where
  E: Send + Sync + 'static,
{
  fn transform<T: Clone + Send + Sync + 'static>(fa: Result<T, E>) -> Effect<T, E> {
    match fa {
      Ok(value) => Effect::new(async move { Ok(value) }),
      Err(err) => Effect::new(async move { Err(err) }),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::time::Duration;

  // Test functions for Functor/Applicative/Monad
  const FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  // Test functions for Alternative
  const ALTERNATIVE_FUNCTIONS: &[fn(i32) -> Result<i32, &'static str>] = &[
    |x| Ok(x + 1),
    |x| Ok(x * 2),
    |x| Err("error"),
    |x| Ok(x - 1),
    |x| Err("failure"),
  ];

  // Test functions for Bifunctor
  const BIFUNCTOR_FUNCTIONS: &[fn(i32) -> i32] = &[|x| x + 1, |x| x * 2, |x| x - 1];

  const BIFUNCTOR_ERROR_FUNCTIONS: &[fn(&'static str) -> &'static str] = &[
    |e| "new error",
    |e| "different error",
    |e| "transformed error",
  ];

  // Test functions for Category
  const CATEGORY_FUNCTIONS: &[fn(i32) -> i32] = &[|x| x + 1, |x| x * 2, |x| x - 1];

  // Test functions for Comonad
  const COMONAD_FUNCTIONS: &[fn(i32) -> i32] = &[|x| x + 1, |x| x * 2, |x| x - 1];

  // Test functions for Natural
  const NATURAL_FUNCTIONS: &[fn(i32) -> i32] = &[|x| x + 1, |x| x * 2, |x| x - 1];

  proptest! {
    #[test]
    fn test_functor_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mut mapped = effect.map(|x| x);
      let result = futures::executor::block_on(async { mapped.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_functor_composition(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];
      let effect1: Effect<i32, &'static str> = Effect::pure(x);
      let effect2: Effect<i32, &'static str> = Effect::pure(x);
      let mut mapped1 = effect1.map(f).map(g);
      let mut mapped2 = effect2.map(|x| g(f(x)));
      let result1 = futures::executor::block_on(async { mapped1.run().await.unwrap() });
      let result2 = futures::executor::block_on(async { mapped2.run().await.unwrap() });
      assert_eq!(result1, result2);
    }

    #[test]
    fn test_applicative_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mut applied = effect.ap(Effect::pure(|x| x));
      let result = futures::executor::block_on(async { applied.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_applicative_composition(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];
      let effect1: Effect<i32, &'static str> = Effect::pure(x);
      let effect2: Effect<i32, &'static str> = Effect::pure(x);
      let mut applied1 = effect1.ap(Effect::pure(f)).ap(Effect::pure(g));
      let mut applied2 = effect2.ap(Effect::pure(|x| g(f(x))));
      let result1 = futures::executor::block_on(async { applied1.run().await.unwrap() });
      let result2 = futures::executor::block_on(async { applied2.run().await.unwrap() });
      assert_eq!(result1, result2);
    }

    #[test]
    fn test_monad_left_identity(x in any::<i32>(), f_idx in 0..FUNCTIONS.len()) {
      let f = FUNCTIONS[f_idx];
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mut bound = effect.bind(|x| Effect::pure(f(x)));
      let result = futures::executor::block_on(async { bound.run().await.unwrap() });
      assert_eq!(result, f(x));
    }

    #[test]
    fn test_monad_right_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mut bound = effect.bind(Effect::pure);
      let result = futures::executor::block_on(async { bound.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_monad_associativity(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let g = FUNCTIONS[g_idx];
      let effect1: Effect<i32, &'static str> = Effect::pure(x);
      let effect2: Effect<i32, &'static str> = Effect::pure(x);
      let mut bound1 = effect1.bind(|x| Effect::pure(f(x))).bind(|x| Effect::pure(g(x)));
      let mut bound2 = effect2.bind(|x| Effect::pure(f(x)).bind(|y| Effect::pure(g(y))));
      let result1 = futures::executor::block_on(async { bound1.run().await.unwrap() });
      let result2 = futures::executor::block_on(async { bound2.run().await.unwrap() });
      assert_eq!(result1, result2);
    }

    #[test]
    fn test_alternative_left_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mut alt = Effect::<i32, &'static str>::empty().alt(effect);
      let result = futures::executor::block_on(async { alt.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_alternative_right_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mut alt = effect.alt(Effect::<i32, &'static str>::empty());
      let result = futures::executor::block_on(async { alt.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_alternative_associativity(
      x in any::<i32>(),
      f_idx in 0..ALTERNATIVE_FUNCTIONS.len(),
      g_idx in 0..ALTERNATIVE_FUNCTIONS.len()
    ) {
      let f = ALTERNATIVE_FUNCTIONS[f_idx];
      let g = ALTERNATIVE_FUNCTIONS[g_idx];
      let effect1: Effect<i32, &'static str> = Effect::new(async move { f(x) });
      let effect2: Effect<i32, &'static str> = Effect::new(async move { g(x) });
      let effect3: Effect<i32, &'static str> = Effect::new(async move { f(x) });
      let effect4: Effect<i32, &'static str> = Effect::new(async move { f(x) });
      let effect5: Effect<i32, &'static str> = Effect::new(async move { g(x) });
      let effect6: Effect<i32, &'static str> = Effect::new(async move { f(x) });
      let mut alt1 = effect1.alt(effect2).alt(effect3);
      let mut alt2 = effect4.alt(effect5.alt(effect6));
      let result1 = futures::executor::block_on(async { alt1.run().await });
      let result2 = futures::executor::block_on(async { alt2.run().await });
      assert_eq!(result1, result2);
    }

    #[test]
    fn test_bifunctor_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let mapped = effect.bimap(|x| x, |e| e);
      let result = futures::executor::block_on(async { mapped.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_bifunctor_composition(
      x in any::<i32>(),
      f_idx in 0..BIFUNCTOR_FUNCTIONS.len(),
      g_idx in 0..BIFUNCTOR_FUNCTIONS.len(),
      h_idx in 0..BIFUNCTOR_ERROR_FUNCTIONS.len(),
      i_idx in 0..BIFUNCTOR_ERROR_FUNCTIONS.len()
    ) {
      let f = BIFUNCTOR_FUNCTIONS[f_idx];
      let g = BIFUNCTOR_FUNCTIONS[g_idx];
      let h = BIFUNCTOR_ERROR_FUNCTIONS[h_idx];
      let i = BIFUNCTOR_ERROR_FUNCTIONS[i_idx];
      let effect1: Effect<i32, &'static str> = Effect::pure(x);
      let effect2: Effect<i32, &'static str> = Effect::pure(x);
      let mapped1 = effect1.bimap(f, h).bimap(g, i);
      let mapped2 = effect2.bimap(|x| g(f(x)), |e| i(h(e)));
      let result1 = futures::executor::block_on(async { mapped1.run().await.unwrap() });
      let result2 = futures::executor::block_on(async { mapped2.run().await.unwrap() });
      assert_eq!(result1, result2);
    }

    #[test]
    fn test_category_identity(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let composed = Effect::<i32, &'static str>::id().compose(effect);
      let result = futures::executor::block_on(async { composed.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_category_associativity(
      x in any::<i32>(),
      f_idx in 0..CATEGORY_FUNCTIONS.len(),
      g_idx in 0..CATEGORY_FUNCTIONS.len(),
      h_idx in 0..CATEGORY_FUNCTIONS.len()
    ) {
      let f = CATEGORY_FUNCTIONS[f_idx];
      let g = CATEGORY_FUNCTIONS[g_idx];
      let h = CATEGORY_FUNCTIONS[h_idx];
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let composed1 = Effect::arr(f).compose(Effect::arr(g)).compose(Effect::arr(h));
      let composed2 = Effect::arr(f).compose(Effect::arr(g).compose(Effect::arr(h)));
      let result1 = futures::executor::block_on(async { composed1.run().await.unwrap() });
      let result2 = futures::executor::block_on(async { composed2.run().await.unwrap() });
      assert_eq!(result1, result2);
    }

    #[test]
    fn test_comonad_extract(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let extracted = effect.extract();
      assert_eq!(extracted, x);
    }

    #[test]
    fn test_comonad_duplicate(x in any::<i32>()) {
      let effect: Effect<i32, &'static str> = Effect::pure(x);
      let duplicated = effect.duplicate();
      let result = futures::executor::block_on(async { duplicated.run().await.unwrap() });
      assert_eq!(result, x);
    }

    #[test]
    fn test_natural_transformation(
      x in any::<i32>(),
      f_idx in 0..NATURAL_FUNCTIONS.len()
    ) {
      let f = NATURAL_FUNCTIONS[f_idx];
      let option = Some(x);
      let effect = Effect::<i32, &'static str>::transform(option);
      let result = futures::executor::block_on(async { effect.run().await.unwrap() });
      assert_eq!(result, x);
    }
  }
}
