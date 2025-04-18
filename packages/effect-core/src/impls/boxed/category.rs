use crate::{traits::Category, threadsafe::CloneableThreadSafe};
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
      let (a, c): &(A, C) = &*x;
      let b = f.apply(Box::new(a.clone()));
      Box::new(((*b).clone(), c.clone()))
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    BoxFn::new(move |x: Box<(C, A)>| {
      let (c, a): &(C, A) = &*x;
      let b = f.apply(Box::new(a.clone()));
      Box::new((c.clone(), (*b).clone()))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::Category;
  use proptest::prelude::*;

  // Define test functions that operate on integers
  // Each function takes an i32 reference and returns an i32
  const INT_FUNCTIONS: &[fn(&i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          x in any::<i32>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Box<i32> as Category<i32, i32>>::arr(f);

          // Get the identity morphism
          let id = <Box<i32> as Category<i32, i32>>::id();

          // Compose id . f
          let id_then_f = <Box<i32> as Category<i32, i32>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Box<i32> as Category<i32, i32>>::compose(arr_f.clone(), id);

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
          x in any::<i32>(),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Box<i32> as Category<i32, i32>>::arr(f);
          let arr_g = <Box<i32> as Category<i32, i32>>::arr(g);
          let arr_h = <Box<i32> as Category<i32, i32>>::arr(h);

          // Compose (f . g) . h
          let fg = <Box<i32> as Category<i32, i32>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Box<i32> as Category<i32, i32>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Box<i32> as Category<i32, i32>>::compose(arr_g, arr_h);
          let f_gh = <Box<i32> as Category<i32, i32>>::compose(arr_f, gh);

          // Apply the input
          let x_box = Box::new(x);
          let result_fg_h = fg_h.apply(x_box.clone());
          let result_f_gh = f_gh.apply(x_box);

          // Both compositions should give the same result
          assert_eq!(*result_fg_h, *result_f_gh);
      }
  }

  // Test the first combinator
  proptest! {
      #[test]
      fn test_first_combinator(
          x in any::<i32>(),
          c in any::<i32>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Box<i32> as Category<i32, i32>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Box<i32> as Category<i32, i32>>::first(arr_f);

          // Create a pair input
          let pair = Box::new((x, c));

          // Apply the first combinator
          let result = first_f.apply(pair);

          // The result should be (f(x), c)
          assert_eq!(result.0, f(&x));
          assert_eq!(result.1, c);
      }
  }

  // Test the second combinator
  proptest! {
      #[test]
      fn test_second_combinator(
          x in any::<i32>(),
          c in any::<i32>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Box<i32> as Category<i32, i32>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Box<i32> as Category<i32, i32>>::second(arr_f);

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
