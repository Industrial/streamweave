use super::category::{LinkedListCategory, LinkedListFn};
use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe, U: CloneableThreadSafe> Arrow<T, U> for LinkedListCategory {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    LinkedListFn::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<T, U>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(T, C), (U, D)> {
    LinkedListFn::new(move |(t, c)| (f.apply(t), g.apply(c)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<T, U>,
    g: Self::Morphism<T, C>,
  ) -> Self::Morphism<T, (U, C)> {
    LinkedListFn::new(move |t: T| (f.apply(t.clone()), g.apply(t)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use proptest::prelude::*;

  #[test]
  fn test_arrow_creation() {
    let f = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x * 2);
    let input = 5;
    let result = f.apply(input);
    assert_eq!(result, 10);
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = 5;

    let f_arrow = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x * 2);
    let f_arr = <LinkedListCategory as Category<i32, i32>>::arr(|x: &i32| x * 2);

    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);

    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x * 2);
    let g = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x + 1);

    let split_fn = <LinkedListCategory as Arrow<i32, i32>>::split::<i32, i32, i32, i32>(f, g);
    let input = (1, 10);

    let result = split_fn.apply(input);
    assert_eq!(result, (2, 11));
  }

  #[test]
  fn test_fanout() {
    let f = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x * 2);
    let g = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x + 1);

    let fanout_fn = <LinkedListCategory as Arrow<i32, i32>>::fanout(f, g);
    let input = 5;

    let result = fanout_fn.apply(input);
    assert_eq!(result, (10, 6));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      x in any::<i32>(),
      y in any::<i32>()
    ) {
      let f = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x.saturating_mul(2));
      let g = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x.saturating_add(1));

      let split_fn = <LinkedListCategory as Arrow<i32, i32>>::split::<i32, i32, i32, i32>(f, g);
      let input = (x, y);

      let result = split_fn.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, x.saturating_mul(2));
      assert_eq!(result.1, y.saturating_add(1));
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      x in any::<i32>()
    ) {
      let f = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x.saturating_mul(2));
      let g = <LinkedListCategory as Arrow<i32, i32>>::arrow(|x: i32| x.saturating_add(1));

      let fanout_fn = <LinkedListCategory as Arrow<i32, i32>>::fanout(f, g);
      let input = x;

      let result = fanout_fn.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, x.saturating_mul(2));
      assert_eq!(result.1, x.saturating_add(1));
    }
  }
}
