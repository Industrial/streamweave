use crate::{traits::Category, threadsafe::CloneableThreadSafe};
use std::sync::Arc;

// A cloneable function wrapper for Iterator
#[derive(Clone)]
pub struct IterFn<A, B>(
  Arc<dyn Fn(Box<dyn Iterator<Item = A>>) -> Box<dyn Iterator<Item = B>> + Send + Sync + 'static>,
);

impl<A, B> IterFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Box<dyn Iterator<Item = A>>) -> Box<dyn Iterator<Item = B>> + Send + Sync + 'static,
  {
    IterFn(Arc::new(f))
  }

  pub fn apply(&self, a: Box<dyn Iterator<Item = A>>) -> Box<dyn Iterator<Item = B>> {
    (self.0)(a)
  }
}

// This is a dummy type just to represent the Iterator category
// We can't directly implement traits for the Iterator trait itself
pub struct IteratorCategory<T>(std::marker::PhantomData<T>);

impl<T: CloneableThreadSafe + 'static> Category<T, T> for IteratorCategory<T> {
  type Morphism<A: CloneableThreadSafe + 'static, B: CloneableThreadSafe + 'static> = IterFn<A, B>;

  fn id<A: CloneableThreadSafe + 'static>() -> Self::Morphism<A, A> {
    IterFn::new(|iter| iter)
  }

  fn compose<
    A: CloneableThreadSafe + 'static,
    B: CloneableThreadSafe + 'static,
    C: CloneableThreadSafe + 'static,
  >(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    IterFn::new(move |iter| g.apply(f.apply(iter)))
  }

  fn arr<A: CloneableThreadSafe + 'static, B: CloneableThreadSafe + 'static, F>(
    f: F,
  ) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    let f = Arc::new(f);
    IterFn::new(move |iter: Box<dyn Iterator<Item = A>>| {
      let f_clone = Arc::clone(&f);
      Box::new(iter.map(move |a| (f_clone)(&a)))
    })
  }

  fn first<
    A: CloneableThreadSafe + 'static,
    B: CloneableThreadSafe + 'static,
    C: CloneableThreadSafe + 'static,
  >(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    IterFn::new(move |iter: Box<dyn Iterator<Item = (A, C)>>| {
      // Split the iterator into components
      let mut a_vec = Vec::new();
      let mut c_vec = Vec::new();

      for (a, c) in iter {
        a_vec.push(a);
        c_vec.push(c);
      }

      // Apply f to the iterator of As
      let b_iter = f.apply(Box::new(a_vec.into_iter()));

      // Collect the Bs
      let mut b_vec = Vec::new();
      for b in b_iter {
        b_vec.push(b);
      }

      // Check lengths match
      let len = std::cmp::min(b_vec.len(), c_vec.len());

      // Create the result iterator by zipping
      Box::new(b_vec.into_iter().zip(c_vec).take(len))
    })
  }

  fn second<
    A: CloneableThreadSafe + 'static,
    B: CloneableThreadSafe + 'static,
    C: CloneableThreadSafe + 'static,
  >(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    IterFn::new(move |iter: Box<dyn Iterator<Item = (C, A)>>| {
      // Split the iterator into components
      let mut c_vec = Vec::new();
      let mut a_vec = Vec::new();

      for (c, a) in iter {
        c_vec.push(c);
        a_vec.push(a);
      }

      // Apply f to the iterator of As
      let b_iter = f.apply(Box::new(a_vec.into_iter()));

      // Collect the Bs
      let mut b_vec = Vec::new();
      for b in b_iter {
        b_vec.push(b);
      }

      // Check lengths match
      let len = std::cmp::min(c_vec.len(), b_vec.len());

      // Create the result iterator by zipping
      Box::new(c_vec.into_iter().zip(b_vec).take(len))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::Category;
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

  // Helper to create a boxed iterator
  fn boxed_iter<T: 'static>(items: Vec<T>) -> Box<dyn Iterator<Item = T>> {
    Box::new(items.into_iter())
  }

  // Helper to collect a boxed iterator
  fn collect_iter<T>(iter: Box<dyn Iterator<Item = T>>) -> Vec<T> {
    iter.collect()
  }

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          xs in proptest::collection::vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <IteratorCategory<i32> as Category<i32, i32>>::arr(f);

          // Get the identity morphism
          let id = <IteratorCategory<i32> as Category<i32, i32>>::id();

          // Compose id . f
          let id_then_f = <IteratorCategory<i32> as Category<i32, i32>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <IteratorCategory<i32> as Category<i32, i32>>::compose(arr_f.clone(), id);

          // Apply the input to each composition
          let result_f = collect_iter(arr_f.apply(boxed_iter(xs.clone())));
          let result_id_then_f = collect_iter(id_then_f.apply(boxed_iter(xs.clone())));
          let result_f_then_id = collect_iter(f_then_id.apply(boxed_iter(xs.clone())));

          // All should give the same result
          assert_eq!(result_f, result_id_then_f);
          assert_eq!(result_f, result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          xs in proptest::collection::vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <IteratorCategory<i32> as Category<i32, i32>>::arr(f);
          let arr_g = <IteratorCategory<i32> as Category<i32, i32>>::arr(g);
          let arr_h = <IteratorCategory<i32> as Category<i32, i32>>::arr(h);

          // Compose (f . g) . h
          let fg = <IteratorCategory<i32> as Category<i32, i32>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <IteratorCategory<i32> as Category<i32, i32>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <IteratorCategory<i32> as Category<i32, i32>>::compose(arr_g, arr_h);
          let f_gh = <IteratorCategory<i32> as Category<i32, i32>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = collect_iter(fg_h.apply(boxed_iter(xs.clone())));
          let result_f_gh = collect_iter(f_gh.apply(boxed_iter(xs.clone())));

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator
  proptest! {
      #[test]
      fn test_first_combinator(
          xs in proptest::collection::vec(any::<i32>(), 0..10),
          cs in proptest::collection::vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Make sure both vectors have the same length
          let min_len = std::cmp::min(xs.len(), cs.len());
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Create pairs
          let pairs: Vec<(i32, i32)> = xs.iter().cloned().zip(cs.iter().cloned()).collect();

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <IteratorCategory<i32> as Category<i32, i32>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <IteratorCategory<i32> as Category<i32, i32>>::first(arr_f);

          // Apply the first combinator
          let result = collect_iter(first_f.apply(boxed_iter(pairs.clone())));

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
          xs in proptest::collection::vec(any::<i32>(), 0..10),
          cs in proptest::collection::vec(any::<i32>(), 0..10),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Make sure both vectors have the same length
          let min_len = std::cmp::min(xs.len(), cs.len());
          let xs = xs.into_iter().take(min_len).collect::<Vec<_>>();
          let cs = cs.into_iter().take(min_len).collect::<Vec<_>>();

          // Create pairs
          let pairs: Vec<(i32, i32)> = cs.iter().cloned().zip(xs.iter().cloned()).collect();

          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <IteratorCategory<i32> as Category<i32, i32>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <IteratorCategory<i32> as Category<i32, i32>>::second(arr_f);

          // Apply the second combinator
          let result = collect_iter(second_f.apply(boxed_iter(pairs.clone())));

          // Compute the expected result
          let expected: Vec<(i32, i32)> = cs.iter().cloned().zip(xs.iter().map(|x| f(x))).collect();

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
