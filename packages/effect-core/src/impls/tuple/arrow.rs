//! Implementation of the `Arrow` trait for tuple types.
//!
//! This module provides arrow operations for tuple types, enabling
//! working with product types in a functional way.

use crate::impls::tuple::category::TupleMorphism;
use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Arrow<B, B> for (A, B) {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    TupleMorphism::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    G: CloneableThreadSafe,
  >(
    f: Self::Morphism<B, B>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(B, C), (B, D)> {
    TupleMorphism::new(move |(x, y)| (f.apply(x), g.apply(y)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<B, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<B, (B, C)> {
    TupleMorphism::new(move |x: B| {
      let x_clone = x.clone();
      (f.apply(x), g.apply(x_clone))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::category::Category;
  use proptest::prelude::*;

  // Define test functions for i64
  const INT_FUNCTIONS: &[fn(i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  #[test]
  fn test_arrow_creation() {
    // Test creating an arrow for a simple function
    let add_one = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| x + 1);
    let result = add_one.apply(5);
    assert_eq!(result, 6);

    // Test with a different function
    let double = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| x * 2);
    let result = double.apply(5);
    assert_eq!(result, 10);
  }

  #[test]
  fn test_split() {
    let add_one = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| x + 1);
    let double = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| x * 2);

    let split = <(i32, i64) as Arrow<i64, i64>>::split::<i64, i64, (), ()>(add_one, double);

    let pair = (5, 10);
    let result = split.apply(pair);
    assert_eq!(result, (6, 20)); // (5+1, 10*2)
  }

  #[test]
  fn test_fanout() {
    let add_one = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| x + 1);
    let double = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| x * 2);

    let fanout = <(i32, i64) as Arrow<i64, i64>>::fanout(add_one, double);

    let value = 5;
    let result = fanout.apply(value);
    assert_eq!(result, (6, 10)); // (5+1, 5*2)
  }

  proptest! {
    #[test]
    fn test_arrow_split_law_prop(
      x in -1000..1000i64,
      y in -1000..1000i64,
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <(i32, i64) as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <(i32, i64) as Arrow<i64, i64>>::arrow(g);

      // Split the arrows
      let split_arrows = <(i32, i64) as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f.clone(), arrow_g.clone());

      // Creating a direct splitting function
      let direct_split_fn = move |pair: (i64, i64)| (f(pair.0), g(pair.1));
      let direct_split = <(i32, (i64, i64)) as Arrow<(i64, i64), (i64, i64)>>::arrow(direct_split_fn);

      // Test with specific input
      let pair = (x, y);
      let result1 = split_arrows.apply(pair);
      let result2 = direct_split.apply(pair);

      // Both approaches should yield the same result
      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn test_arrow_fanout_law_prop(
      x in -1000..1000i64,
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <(i32, i64) as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <(i32, i64) as Arrow<i64, i64>>::arrow(g);

      // Fanout implementation
      let fanout_arrows = <(i32, i64) as Arrow<i64, i64>>::fanout(arrow_f.clone(), arrow_g.clone());

      // Alternative implementation using split and duplicate
      let duplicate = <(i32, i64) as Arrow<i64, i64>>::arrow(|x: i64| (x, x));
      let split_arrows = <(i32, i64) as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f, arrow_g);
      let compose_result = <(i32, i64) as Category<i64, i64>>::compose(duplicate, split_arrows);

      // Test with specific input
      let result1 = fanout_arrows.apply(x);
      let result2 = compose_result.apply(x);

      // Both approaches should yield the same result
      prop_assert_eq!(result1, result2);
    }
  }

  #[test]
  fn test_arrow_laws() {
    // Test the arrow law: arr(id) = id
    let identity = |x: i64| x;
    let arrow_id = <(i32, i64) as Arrow<i64, i64>>::arrow(identity);
    let category_id = <(i32, i64) as Category<i64, i64>>::id();

    let x = 42;
    assert_eq!(arrow_id.apply(x), category_id.apply(x));

    // Test law: arr(f >>> g) = arr(f) >>> arr(g)
    let f = |x: i64| x + 1;
    let g = |x: i64| x * 2;
    let fg = move |x: i64| g(f(x));

    let arrow_f = <(i32, i64) as Arrow<i64, i64>>::arrow(f);
    let arrow_g = <(i32, i64) as Arrow<i64, i64>>::arrow(g);
    let arrow_fg = <(i32, i64) as Arrow<i64, i64>>::arrow(fg);

    let composed = <(i32, i64) as Category<i64, i64>>::compose(arrow_f, arrow_g);

    let x = 5;
    assert_eq!(arrow_fg.apply(x), composed.apply(x));
  }
}
