//! Profunctor trait and implementations.
//!
//! A profunctor is a type that is both covariant in its second type parameter
//! and contravariant in its first type parameter.

/// A profunctor is a type that is both covariant in its second type parameter
/// and contravariant in its first type parameter.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter A must implement Send + Sync + 'static
/// - The type parameter B must implement Send + Sync + 'static
/// - The type parameter C must implement Send + Sync + 'static
/// - The type parameter D must implement Send + Sync + 'static
/// - The functions f and g must implement Send + Sync + 'static
pub trait Profunctor<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  type HigherSelf<C: Send + Sync + 'static, D: Send + Sync + 'static>: Profunctor<C, D>;

  /// Maps over both type parameters of the profunctor.
  fn dimap<C: Send + Sync + 'static, D: Send + Sync + 'static, F, G>(
    self,
    f: F,
    g: G,
  ) -> Self::HigherSelf<C, D>
  where
    F: Fn(C) -> A + Send + Sync + 'static,
    G: Fn(B) -> D + Send + Sync + 'static;

  /// Maps over the first type parameter (contravariant).
  fn lmap<C: Send + Sync + 'static, F>(self, f: F) -> Self::HigherSelf<C, B>
  where
    F: Fn(C) -> A + Send + Sync + 'static,
  {
    self.dimap(f, |x| x)
  }

  /// Maps over the second type parameter (covariant).
  fn rmap<D: Send + Sync + 'static, G>(self, g: G) -> Self::HigherSelf<A, D>
  where
    G: Fn(B) -> D + Send + Sync + 'static,
  {
    self.dimap(|x| x, g)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::function::Function;
  use crate::pair::Pair;
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

  // Helper function to test thread safety
  fn test_thread_safety<T: Profunctor<i32, i32> + Send + Sync + 'static>(
    value: T,
    f_idx: usize,
    g_idx: usize,
  ) where
    T::HigherSelf<i32, i32>: Send + Sync + 'static,
  {
    let f = FUNCTIONS[f_idx];
    let g = FUNCTIONS[g_idx];
    let handle = std::thread::spawn(move || value.dimap(f, g));
    assert!(handle.join().is_ok());
  }

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
    fn test_thread_safety_function(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      test_thread_safety(Function::new(|x: i32| x + 1), f_idx, g_idx);
    }

    #[test]
    fn test_thread_safety_pair(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len(),
      g_idx in 0..FUNCTIONS.len()
    ) {
      test_thread_safety(Pair::new(|x: i32| x + 1, |x: i32| x - 1), f_idx, g_idx);
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
    fn test_function_contravariant(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let function = Function::new(|x: i32| x + 1);
      let contramapped = function.contramap(f);
      assert_eq!(contramapped.apply(x), function.apply(f(x)));
    }

    #[test]
    fn test_pair_contravariant(
      x in any::<i32>(),
      f_idx in 0..FUNCTIONS.len()
    ) {
      let f = FUNCTIONS[f_idx];
      let pair = Pair::new(|x: i32| x + 1, |x: i32| x - 1);
      let contramapped = pair.contramap(f);
      assert_eq!(contramapped.apply(x), pair.apply(f(x)));
    }
  }
}
