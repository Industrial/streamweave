use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use super::category::BoolFn;

impl Arrow<bool, bool> for bool {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    BoolFn::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<bool, bool>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(bool, C), (bool, D)> {
    BoolFn::new(move |(b, c)| (f.apply(b), g.apply(c)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<bool, bool>,
    g: Self::Morphism<bool, C>,
  ) -> Self::Morphism<bool, (bool, C)> {
    BoolFn::new(move |b: bool| (f.apply(b.clone()), g.apply(b)))
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
    let f = bool::arrow(|x: bool| !x);
    let input = true;
    let result = f.apply(input);
    assert_eq!(result, false);
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = true;
    
    let f_arrow = bool::arrow(|x: bool| !x);
    let f_arr = bool::arr(|x: &bool| !x);
    
    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);
    
    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = bool::arrow(|x: bool| !x);
    let g = bool::arrow(|x: i32| x + 1);
    
    let split_fn = bool::split::<i32, i32, i32, i32>(f, g);
    let input = (true, 10);
    
    let result = split_fn.apply(input);
    assert_eq!(result, (false, 11));
  }

  #[test]
  fn test_fanout() {
    let f = bool::arrow(|x: bool| !x);
    let g = bool::arrow(|x: bool| x.to_string());
    
    let fanout_fn = bool::fanout(f, g);
    let input = true;
    
    let result = fanout_fn.apply(input);
    assert_eq!(result, (false, "true".to_string()));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      b in any::<bool>(),
      y in any::<i32>()
    ) {
      let f = bool::arrow(|x: bool| !x);
      let g = bool::arrow(|x: i32| x.saturating_add(1));
      
      let split_fn = bool::split::<i32, i32, i32, i32>(f, g);
      let input = (b, y);
      
      let result = split_fn.apply(input);
      
      // Check that the transformation is correct
      assert_eq!(result.0, !b);
      assert_eq!(result.1, y.saturating_add(1));
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      b in any::<bool>()
    ) {
      let f = bool::arrow(|x: bool| !x);
      let g = bool::arrow(|x: bool| x.to_string());
      
      let fanout_fn = bool::fanout(f, g);
      let input = b;
      
      let result = fanout_fn.apply(input);
      
      // Check that the transformation is correct
      assert_eq!(result.0, !b);
      assert_eq!(result.1, b.to_string());
    }
  }
} 