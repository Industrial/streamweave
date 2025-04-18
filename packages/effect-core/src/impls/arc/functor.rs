use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

impl<A: CloneableThreadSafe> Functor<A> for Arc<A> {
  type HigherSelf<B: CloneableThreadSafe> = Arc<B>;

  fn map<B, F>(self, mut f: F) -> Self::HigherSelf<B>
  where
    F: for<'a> FnMut(&'a A) -> B + CloneableThreadSafe,
    B: CloneableThreadSafe,
  {
    Arc::new(f(&self))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;
  use std::thread;

  // Define test functions with overflow protection
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Helper for testing thread safety
  fn test_thread_safety<F, T, U>(f: F, input: T) -> U
  where
    F: FnOnce(T) -> U + Send + 'static,
    T: Send + 'static,
    U: Send + 'static,
  {
    let handle = thread::spawn(move || f(input));
    handle.join().unwrap()
  }

  #[test]
  fn test_identity_law() {
    let x = 42i64;
    let arc = Arc::new(x);
    let id = |x: &i64| *x;
    let result = arc.map(id);

    // Identity law: functor.map(id) == functor
    assert_eq!(*result, x);
  }

  proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn test_identity_law_prop(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      let arc = Arc::new(x);
      let id = |x: &i64| *x;
      let result = arc.map(id);

      // Identity law: functor.map(id) == functor
      prop_assert_eq!(*result, x);
    }

    #[test]
    fn test_composition_law(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      // Get functions from array
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      // Create clones since map consumes self
      let arc1 = Arc::new(x);
      let arc2 = Arc::new(x);

      // Apply f then g - copy the functions to allow moving in closures
      let f1 = f;
      let g1 = g;
      let result1 = arc1.map(f1).map(g1);

      // Apply composition of f and g - copy the functions to allow moving in closures
      let f2 = f;
      let g2 = g;
      let result2 = arc2.map(move |x| {
        let intermediate = f2(x);
        g2(&intermediate)
      });

      // Composition law: functor.map(f).map(g) == functor.map(g ∘ f)
      prop_assert_eq!(*result1, *result2);
    }

    // Implementation-specific test: thread safety and shared ownership
    #[test]
    fn test_arc_thread_safety(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let input_f = INT_FUNCTIONS[f_idx];
      let arc = Arc::new(x);

      // Make a clone for verification later
      let arc_clone = arc.clone();

      // Map in a separate thread - need to capture the function
      let f_captured = input_f;
      let result = test_thread_safety(move |a| a.map(f_captured), arc);

      // The result should match applying f to x
      prop_assert_eq!(*result, f_captured(&x));

      // The original Arc should still be valid and have the original value
      prop_assert_eq!(*arc_clone, x);
    }

    // Test that Arc can be shared and mapped multiple times
    #[test]
    fn test_arc_shared_ownership(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arc1 = Arc::new(x);
      let arc2 = arc1.clone();

      // Map one clone with f
      let result1 = arc1.map(f);

      // Map the other clone with g
      let result2 = arc2.map(g);

      // Both results should be correctly mapped
      prop_assert_eq!(*result1, f(&x));
      prop_assert_eq!(*result2, g(&x));
    }
  }
}
