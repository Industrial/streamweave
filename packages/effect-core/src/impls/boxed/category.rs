use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::sync::Arc;

// A cloneable function wrapper for boxed functions
#[derive(Clone)]
pub struct BoxFn<A, B>(Arc<dyn Fn(Box<A>) -> Box<B> + Send + Sync + 'static>);

impl<A, B> BoxFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Box<A>) -> Box<B> + Send + Sync + 'static,
  {
    BoxFn(Arc::new(f))
  }

  pub fn apply(&self, a: Box<A>) -> Box<B> {
    (self.0)(a)
  }
}

impl<T: CloneableThreadSafe> Category<T, T> for Box<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = BoxFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    BoxFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    BoxFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    BoxFn::new(move |x| {
      let inner = &*x; // Borrow the value inside the Box
      Box::new(f(inner)) // Apply the closure
    })
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    BoxFn::new(move |x: Box<(A, C)>| {
      let (a, c): &(A, C) = &x;
      let b = f.apply(Box::new(a.clone()));
      Box::new(((*b).clone(), c.clone()))
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    BoxFn::new(move |x: Box<(C, A)>| {
      let (c, a): &(C, A) = &x;
      let b = f.apply(Box::new(a.clone()));
      Box::new((c.clone(), (*b).clone()))
    })
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::category::Category;
  use proptest::prelude::*;

  // Define test functions that operate on integers - using i64 instead of i32
  // and using checked arithmetic to prevent overflow
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Box<i64> as Category<i64, i64>>::arr(f);

          // Get the identity morphism
          let id = <Box<i64> as Category<i64, i64>>::id();

          // Compose id . f
          let id_then_f = <Box<i64> as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Box<i64> as Category<i64, i64>>::compose(arr_f.clone(), id);

          // Apply the input to each composition
          let x_box = Box::new(x);
          let result_f = arr_f.apply(x_box.clone());
          let result_id_then_f = id_then_f.apply(x_box.clone());
          let result_f_then_id = f_then_id.apply(x_box);

          // All should give the same result
          assert_eq!(*result_f, *result_id_then_f);
          assert_eq!(*result_f, *result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Box<i64> as Category<i64, i64>>::arr(f);
          let arr_g = <Box<i64> as Category<i64, i64>>::arr(g);
          let arr_h = <Box<i64> as Category<i64, i64>>::arr(h);

          // Compose (f . g) . h
          let fg = <Box<i64> as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Box<i64> as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Box<i64> as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <Box<i64> as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input
          let x_box = Box::new(x);
          let result_fg_h = fg_h.apply(x_box.clone());
          let result_f_gh = f_gh.apply(x_box);

          // Both compositions should give the same result
          assert_eq!(*result_fg_h, *result_f_gh);
      }
  }

  // Test the first combinator - specific to Box implementation (owned heap allocation)
  proptest! {
      #[test]
      fn test_first_combinator(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          c in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Box<i64> as Category<i64, i64>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Box<i64> as Category<i64, i64>>::first(arr_f);

          // Create a pair input
          let pair = Box::new((x, c));

          // Apply the first combinator
          let result = first_f.apply(pair);

          // The result should be (f(x), c)
          assert_eq!(result.0, f(&x));
          assert_eq!(result.1, c);
      }
  }

  // Test the second combinator - specific to Box implementation (owned heap allocation)
  proptest! {
      #[test]
      fn test_second_combinator(
          // Use bounded input to prevent overflow
          x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          c in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Box<i64> as Category<i64, i64>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Box<i64> as Category<i64, i64>>::second(arr_f);

          // Create a pair input
          let pair = Box::new((c, x));

          // Apply the second combinator
          let result = second_f.apply(pair);

          // The result should be (c, f(x))
          assert_eq!(result.0, c);
          assert_eq!(result.1, f(&x));
      }
  }
}
