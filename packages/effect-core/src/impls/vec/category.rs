use crate::{traits::Category, threadsafe::CloneableThreadSafe};
use std::sync::Arc;

// A cloneable function wrapper for Vec
#[derive(Clone)]
pub struct VecFn<A, B>(Arc<dyn Fn(Vec<A>) -> Vec<B> + Send + Sync + 'static>);

impl<A, B> VecFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Vec<A>) -> Vec<B> + Send + Sync + 'static,
  {
    VecFn(Arc::new(f))
  }

  pub fn apply(&self, a: Vec<A>) -> Vec<B> {
    (self.0)(a)
  }
}

impl<T: CloneableThreadSafe> Category<T, T> for Vec<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = VecFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    VecFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    VecFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    VecFn::new(move |xs| xs.iter().map(|a| f(a)).collect())
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    VecFn::new(move |xs: Vec<(A, C)>| {
      // Split into components
      let (as_vec, cs_vec): (Vec<A>, Vec<C>) = xs.into_iter().unzip();

      // Apply f to the first component
      let bs_vec = f.apply(as_vec);

      // Zip back together
      bs_vec.into_iter().zip(cs_vec).collect()
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    VecFn::new(move |xs: Vec<(C, A)>| {
      // Split into components
      let (cs_vec, as_vec): (Vec<C>, Vec<A>) = xs.into_iter().unzip();

      // Apply f to the second component
      let bs_vec = f.apply(as_vec);

      // Zip back together
      cs_vec.into_iter().zip(bs_vec).collect()
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::Category;
  use proptest::collection::vec;
  use proptest::prelude::*;

  // Define test functions that operate on integers
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
          xs in vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Vec<i32> as Category<i32, i32>>::arr(f);

          // Get the identity morphism
          let id = <Vec<i32> as Category<i32, i32>>::id();

          // Compose id . f
          let id_then_f = <Vec<i32> as Category<i32, i32>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Vec<i32> as Category<i32, i32>>::compose(arr_f.clone(), id);

          // Apply the input to each composition
          let result_f = arr_f.apply(xs.clone());
          let result_id_then_f = id_then_f.apply(xs.clone());
          let result_f_then_id = f_then_id.apply(xs);

          // All should give the same result
          assert_eq!(result_f, result_id_then_f);
          assert_eq!(result_f, result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          xs in vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Vec<i32> as Category<i32, i32>>::arr(f);
          let arr_g = <Vec<i32> as Category<i32, i32>>::arr(g);
          let arr_h = <Vec<i32> as Category<i32, i32>>::arr(h);

          // Compose (f . g) . h
          let fg = <Vec<i32> as Category<i32, i32>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Vec<i32> as Category<i32, i32>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Vec<i32> as Category<i32, i32>>::compose(arr_g, arr_h);
          let f_gh = <Vec<i32> as Category<i32, i32>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(xs.clone());
          let result_f_gh = f_gh.apply(xs);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator
  proptest! {
      #[test]
      fn test_first_combinator(
          xs in vec(any::<i32>(), 0..10),
          cs in vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Make sure both vectors have the same length
          let min_len = std::cmp::min(xs.len(), cs.len());
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Vec<i32> as Category<i32, i32>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Vec<i32> as Category<i32, i32>>::first(arr_f);

          // Create a vector of pairs
          let pairs: Vec<(i32, i32)> = xs.iter().cloned().zip(cs.iter().cloned()).collect();

          // Apply the first combinator
          let result = first_f.apply(pairs);

          // Compute the expected result
          let expected: Vec<(i32, i32)> = xs.iter().map(|x| f(x)).zip(cs.iter().cloned()).collect();

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator
  proptest! {
      #[test]
      fn test_second_combinator(
          xs in vec(any::<i32>(), 0..10),
          cs in vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Make sure both vectors have the same length
          let min_len = std::cmp::min(xs.len(), cs.len());
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Vec<i32> as Category<i32, i32>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Vec<i32> as Category<i32, i32>>::second(arr_f);

          // Create a vector of pairs
          let pairs: Vec<(i32, i32)> = cs.iter().cloned().zip(xs.iter().cloned()).collect();

          // Apply the second combinator
          let result = second_f.apply(pairs);

          // Compute the expected result
          let expected: Vec<(i32, i32)> = cs.iter().cloned().zip(xs.iter().map(|x| f(x))).collect();

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
