use crate::{threadsafe::CloneableThreadSafe, traits::Category};
use std::sync::Arc;

// A cloneable function wrapper for Option
#[derive(Clone)]
pub struct OptionFn<A, B>(Arc<dyn Fn(Option<A>) -> Option<B> + Send + Sync + 'static>);

impl<A, B> OptionFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Option<A>) -> Option<B> + Send + Sync + 'static,
  {
    OptionFn(Arc::new(f))
  }

  pub fn apply(&self, a: Option<A>) -> Option<B> {
    (self.0)(a)
  }
}

impl<T: CloneableThreadSafe> Category<T, T> for Option<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = OptionFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    OptionFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    OptionFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(&A) -> B + CloneableThreadSafe,
  {
    OptionFn::new(move |x| x.map(|a| f(&a)))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    OptionFn::new(move |x: Option<(A, C)>| {
      x.and_then(|(a, c)| {
        let b = f.apply(Some(a));
        b.map(|b_val| (b_val, c))
      })
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    OptionFn::new(move |x: Option<(C, A)>| {
      x.and_then(|(c, a)| {
        let b = f.apply(Some(a));
        b.map(|b_val| (c, b_val))
      })
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

  // Test the identity law: id() . f = f = f . id()
  proptest! {
      #[test]
      fn test_identity_law(
          opt_val in prop::option::of(any::<i32>()),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create our Category::arr version of the function
          let arr_f = <Option<i32> as Category<i32, i32>>::arr(f);

          // Get the identity morphism
          let id = <Option<i32> as Category<i32, i32>>::id();

          // Compose id . f
          let id_then_f = <Option<i32> as Category<i32, i32>>::compose(id.clone(), arr_f.clone());

          // Compose f . id
          let f_then_id = <Option<i32> as Category<i32, i32>>::compose(arr_f.clone(), id);

          // Apply the input to each composition
          let result_f = arr_f.apply(opt_val.clone());
          let result_id_then_f = id_then_f.apply(opt_val.clone());
          let result_f_then_id = f_then_id.apply(opt_val);

          // All should give the same result
          assert_eq!(result_f, result_id_then_f);
          assert_eq!(result_f, result_f_then_id);
      }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  proptest! {
      #[test]
      fn test_composition_law(
          opt_val in prop::option::of(any::<i32>()),
          f_idx in 0..INT_FUNCTIONS.len(),
          g_idx in 0..INT_FUNCTIONS.len(),
          h_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get functions from our array
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <Option<i32> as Category<i32, i32>>::arr(f);
          let arr_g = <Option<i32> as Category<i32, i32>>::arr(g);
          let arr_h = <Option<i32> as Category<i32, i32>>::arr(h);

          // Compose (f . g) . h
          let fg = <Option<i32> as Category<i32, i32>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <Option<i32> as Category<i32, i32>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <Option<i32> as Category<i32, i32>>::compose(arr_g, arr_h);
          let f_gh = <Option<i32> as Category<i32, i32>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(opt_val.clone());
          let result_f_gh = f_gh.apply(opt_val);

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
      }
  }

  // Test the first combinator
  proptest! {
      #[test]
      fn test_first_combinator(
          x in any::<i32>(),
          c in any::<i32>(),
          is_some in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Option<i32> as Category<i32, i32>>::arr(f);

          // Apply first to get a function on pairs
          let first_f = <Option<i32> as Category<i32, i32>>::first(arr_f);

          // Create an optional pair input
          let pair = if is_some { Some((x, c)) } else { None };

          // Apply the first combinator
          let result = first_f.apply(pair.clone());

          // The expected result
          let expected = pair.map(|(a, c)| (f(&a), c));

          // Results should match
          assert_eq!(result, expected);
      }
  }

  // Test the second combinator
  proptest! {
      #[test]
      fn test_second_combinator(
          x in any::<i32>(),
          c in any::<i32>(),
          is_some in any::<bool>(),
          f_idx in 0..INT_FUNCTIONS.len()
      ) {
          // Get a function from our array
          let f = INT_FUNCTIONS[f_idx];

          // Create the arr version
          let arr_f = <Option<i32> as Category<i32, i32>>::arr(f);

          // Apply second to get a function on pairs
          let second_f = <Option<i32> as Category<i32, i32>>::second(arr_f);

          // Create an optional pair input
          let pair = if is_some { Some((c, x)) } else { None };

          // Apply the second combinator
          let result = second_f.apply(pair.clone());

          // The expected result
          let expected = pair.map(|(c, a)| (c, f(&a)));

          // Results should match
          assert_eq!(result, expected);
      }
  }
}
