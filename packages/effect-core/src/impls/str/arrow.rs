use super::category::{StrCategory, StrFn};
use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;

impl Arrow<String, String> for StrCategory {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    StrFn::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<String, String>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(String, C), (String, D)> {
    StrFn::new(move |(s, c)| (f.apply(s), g.apply(c)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<String, String>,
    g: Self::Morphism<String, C>,
  ) -> Self::Morphism<String, (String, C)> {
    StrFn::new(move |s: String| (f.apply(s.clone()), g.apply(s)))
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
    let f = <StrCategory as Arrow<String, String>>::arrow(|x: i32| x * 2);
    let input = 5;
    let result = f.apply(input);
    assert_eq!(result, 10);
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = 5;

    let f_arrow = <StrCategory as Arrow<String, String>>::arrow(|x: i32| x * 2);
    let f_arr = <StrCategory as Category<String, String>>::arr(|x: &i32| x * 2);

    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);

    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = <StrCategory as Arrow<String, String>>::arrow(|x: String| x.to_uppercase());
    let g = <StrCategory as Arrow<String, String>>::arrow(|x: i32| x + 1);

    let split_fn = <StrCategory as Arrow<String, String>>::split::<i32, i32, i32, i32>(f, g);
    let input = ("hello".to_string(), 10);

    let result = split_fn.apply(input);
    assert_eq!(result, ("HELLO".to_string(), 11));
  }

  #[test]
  fn test_fanout() {
    let f = <StrCategory as Arrow<String, String>>::arrow(|x: String| x.to_uppercase());
    let g = <StrCategory as Arrow<String, String>>::arrow(|x: String| x.len());

    let fanout_fn = <StrCategory as Arrow<String, String>>::fanout(f, g);
    let input = "hello".to_string();

    let result = fanout_fn.apply(input);
    assert_eq!(result, ("HELLO".to_string(), 5));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      s in any::<String>(),
      y in any::<i32>()
    ) {
      let f = <StrCategory as Arrow<String, String>>::arrow(|x: String| x.to_uppercase());
      let g = <StrCategory as Arrow<String, String>>::arrow(|x: i32| x.saturating_add(1));

      let split_fn = <StrCategory as Arrow<String, String>>::split::<i32, i32, i32, i32>(f, g);
      let input = (s.clone(), y);

      let result = split_fn.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, s.to_uppercase());
      assert_eq!(result.1, y.saturating_add(1));
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      s in any::<String>()
    ) {
      let f = <StrCategory as Arrow<String, String>>::arrow(|x: String| x.to_uppercase());
      let g = <StrCategory as Arrow<String, String>>::arrow(|x: String| x.len());

      let fanout_fn = <StrCategory as Arrow<String, String>>::fanout(f, g);
      let input = s.clone();

      let result = fanout_fn.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, s.to_uppercase());
      assert_eq!(result.1, s.len());
    }
  }
}
