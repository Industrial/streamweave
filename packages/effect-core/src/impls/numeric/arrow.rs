use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use super::category::NumericFn;

// Implement Arrow for numeric types using a macro
macro_rules! impl_numeric_arrow {
  ($($t:ty),*) => {
    $(
      impl Arrow<$t, $t> for $t {
        fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
        where
          F: Fn(C) -> D + CloneableThreadSafe,
        {
          NumericFn::new(move |x| f(x))
        }

        fn split<
          C: CloneableThreadSafe,
          D: CloneableThreadSafe,
          E: CloneableThreadSafe,
          F: CloneableThreadSafe,
        >(
          f: Self::Morphism<$t, $t>,
          g: Self::Morphism<C, D>,
        ) -> Self::Morphism<($t, C), ($t, D)> {
          NumericFn::new(move |(n, c)| (f.apply(n), g.apply(c)))
        }

        fn fanout<C: CloneableThreadSafe>(
          f: Self::Morphism<$t, $t>,
          g: Self::Morphism<$t, C>,
        ) -> Self::Morphism<$t, ($t, C)> {
          NumericFn::new(move |n: $t| (f.apply(n.clone()), g.apply(n)))
        }
      }
    )*
  };
}

// Apply the macro to implement Arrow for integer and floating-point types
impl_numeric_arrow!(i32, i64, f32, f64);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use proptest::prelude::*;

  #[test]
  fn test_arrow_creation() {
    let f = i32::arrow(|x: i32| x * 2);
    let input = 5;
    let result = f.apply(input);
    assert_eq!(result, 10);
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = 5;
    
    let f_arrow = i32::arrow(|x: i32| x * 2);
    let f_arr = i32::arr(|x: &i32| x * 2);
    
    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);
    
    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = i32::arrow(|x: i32| x * 2);
    let g = i32::arrow(|x: i32| x + 1);
    
    let split_fn = i32::split::<i32, i32, i32, i32>(f, g);
    let input = (1, 10);
    
    let result = split_fn.apply(input);
    assert_eq!(result, (2, 11));
  }

  #[test]
  fn test_fanout() {
    let f = i32::arrow(|x: i32| x * 2);
    let g = i32::arrow(|x: i32| x + 1);
    
    let fanout_fn = i32::fanout(f, g);
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
      let f = i32::arrow(|x: i32| x.saturating_mul(2));
      let g = i32::arrow(|x: i32| x.saturating_add(1));
      
      let split_fn = i32::split::<i32, i32, i32, i32>(f, g);
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
      let f = i32::arrow(|x: i32| x.saturating_mul(2));
      let g = i32::arrow(|x: i32| x.saturating_add(1));
      
      let fanout_fn = i32::fanout(f, g);
      let input = x;
      
      let result = fanout_fn.apply(input);
      
      // Check that the transformation is correct
      assert_eq!(result.0, x.saturating_mul(2));
      assert_eq!(result.1, x.saturating_add(1));
    }
  }
} 