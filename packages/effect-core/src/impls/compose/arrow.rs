//! Implementation of the `Arrow` trait for `Compose`.
//!
//! This module provides arrow operations for `Compose` types, enabling
//! composable arrows with support for splitting and fanout operations.

use crate::traits::arrow::Arrow;
use crate::types::compose::Compose;
use crate::types::threadsafe::CloneableThreadSafe;

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Arrow<A, B> for Compose<A, B> {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    Compose::new(f, |x| x)
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    G: CloneableThreadSafe,
  >(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(A, C), (B, D)> {
    Compose::new(move |(a, c)| (f.apply(a), g.apply(c)), |x| x)
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<A, C>,
  ) -> Self::Morphism<A, (B, C)> {
    Compose::new(
      move |a: A| {
        let a_clone = a.clone();
        (f.apply(a), g.apply(a_clone))
      },
      |x| x,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::category::Category;
  use proptest::prelude::*;

  // Define test functions for i32 that are safe and easy to reason about
  const I32_FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| x.checked_div(2).unwrap_or(0),
    |x| x.checked_mul(x).unwrap_or(i32::MAX),
    |x| x.checked_neg().unwrap_or(i32::MIN),
  ];

  #[test]
  fn test_arrow_creation() {
    let f = |x: i32| x + 1;
    let arrow_f = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(f);

    let input = 5;
    let result = arrow_f.apply(input);

    assert_eq!(result, 6);
  }

  #[test]
  fn test_split() {
    let f = |x: i32| x + 1;
    let g = |x: i32| x * 2;

    let arrow_f = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(f);
    let arrow_g = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(g);

    let split = <Compose<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(arrow_f, arrow_g);

    let input = (5, 10);
    let result = split.apply(input);

    assert_eq!(result, (6, 20)); // (5+1, 10*2)
  }

  #[test]
  fn test_fanout() {
    let f = |x: i32| x + 1;
    let g = |x: i32| x * 2;

    let arrow_f = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(f);
    let arrow_g = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(g);

    let fanout = <Compose<i32, i32> as Arrow<i32, i32>>::fanout(arrow_f, arrow_g);

    let input = 5;
    let result = fanout.apply(input);

    assert_eq!(result, (6, 10)); // (5+1, 5*2)
  }

  proptest! {
    #[test]
    fn test_arrow_split_law_prop(
      x in -1000..1000i32,
      y in -1000..1000i32,
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];

      let arrow_f = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(f);
      let arrow_g = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(g);

      // Split the arrows
      let split_arrows = <Compose<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(arrow_f.clone(), arrow_g.clone());

      // Creating a direct splitting function
      let direct_split_fn = move |pair: (i32, i32)| (f(pair.0), g(pair.1));
      let direct_split = <Compose<(i32, i32), (i32, i32)> as Arrow<(i32, i32), (i32, i32)>>::arrow(direct_split_fn);

      // Test with specific input
      let pair = (x, y);
      let result1 = split_arrows.apply(pair);
      let result2 = direct_split.apply(pair);

      // Both approaches should yield the same result
      prop_assert_eq!(result1, result2);
    }
  }

  #[test]
  fn test_arrow_laws() {
    // Test the arrow law: arrow(id) = id
    let identity = |x: i32| x;
    let arrow_id = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(identity);
    let category_id = <Compose<i32, i32> as Category<i32, i32>>::id();

    let x = 42;
    assert_eq!(arrow_id.apply(x), category_id.apply(x));

    // Test law: arrow(f >>> g) = arrow(f) >>> arrow(g)
    let f = |x: i32| x + 1;
    let g = |x: i32| x * 2;
    let fg = move |x: i32| g(f(x));

    let arrow_f = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(f);
    let arrow_g = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(g);
    let arrow_fg = <Compose<i32, i32> as Arrow<i32, i32>>::arrow(fg);

    let composed = <Compose<i32, i32> as Category<i32, i32>>::compose(arrow_f, arrow_g);

    let x = 5;
    assert_eq!(arrow_fg.apply(x), composed.apply(x));
  }
}
