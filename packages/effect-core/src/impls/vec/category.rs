use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
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
  use crate::traits::category::Category;
  use proptest::collection::vec;
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
          // Use smaller bounds for inputs
          xs in vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Vec<i64> as Category<i64, i64>>::arr(f);

          // Get the identity morphism
          let id = <Vec<i64> as Category<i64, i64>>::id();

          // Compose id . f
          let id_then_f = <Vec<i64> as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Vec<i64> as Category<i64, i64>>::compose(arr_f.clone(), id);

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
          // Use smaller bounds for inputs
          xs in vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Vec<i64> as Category<i64, i64>>::arr(f);
          let arr_g = <Vec<i64> as Category<i64, i64>>::arr(g);
          let arr_h = <Vec<i64> as Category<i64, i64>>::arr(h);

          // Compose (f . g) . h
          let fg = <Vec<i64> as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Vec<i64> as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Vec<i64> as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <Vec<i64> as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(xs.clone());
          let result_f_gh = f_gh.apply(xs);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator - specific to Vec implementation (working with collections)
  proptest! {
      #[test]
      fn test_first_combinator(
          // Use smaller bounds for inputs
          xs in vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          cs in vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Make sure both vectors have the same length
          let min_len = std::cmp::min(xs.len(), cs.len());
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Vec<i64> as Category<i64, i64>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Vec<i64> as Category<i64, i64>>::first(arr_f);

          // Create a vector of pairs
          let pairs: Vec<(i64, i64)> = xs.iter().cloned().zip(cs.iter().cloned()).collect();

          // Apply the first combinator
          let result = first_f.apply(pairs);

          // Compute the expected result
          let expected: Vec<(i64, i64)> = xs.iter().map(|x| f(x)).zip(cs.iter().cloned()).collect();

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator - specific to Vec implementation (working with collections)
  proptest! {
      #[test]
      fn test_second_combinator(
          // Use smaller bounds for inputs
          xs in vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          cs in vec(any::<i64>().prop_filter("Value too large", |v| *v < 10000), 0..5),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Make sure both vectors have the same length
          let min_len = std::cmp::min(xs.len(), cs.len());
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Vec<i64> as Category<i64, i64>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Vec<i64> as Category<i64, i64>>::second(arr_f);

          // Create a vector of pairs
          let pairs: Vec<(i64, i64)> = cs.iter().cloned().zip(xs.iter().cloned()).collect();

          // Apply the second combinator
          let result = second_f.apply(pairs);

          // Compute the expected result
          let expected: Vec<(i64, i64)> = cs.iter().cloned().zip(xs.iter().map(|x| f(x))).collect();

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
