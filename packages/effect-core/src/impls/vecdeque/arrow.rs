use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use super::category::VecDequeFn;
use std::collections::VecDeque;

impl<T: CloneableThreadSafe> Arrow<T, T> for VecDeque<T> {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    VecDequeFn::new(move |xs| xs.into_iter().map(&f).collect())
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<T, T>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(T, C), (T, D)> {
    VecDequeFn::new(move |xs: VecDeque<(T, C)>| {
      // Split into components
      let (as_vec, cs_vec): (VecDeque<T>, VecDeque<C>) = xs.into_iter().unzip();

      // Apply f to the first component
      let bs_vec = f.apply(as_vec);

      // Apply g to the second component
      let ds_vec = g.apply(cs_vec);

      // Zip back together
      bs_vec.into_iter().zip(ds_vec).collect()
    })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<T, T>,
    g: Self::Morphism<T, C>,
  ) -> Self::Morphism<T, (T, C)> {
    VecDequeFn::new(move |xs: VecDeque<T>| {
      // Apply f to get T values
      let bs_vec = f.apply(xs.clone());

      // Apply g to get C values
      let cs_vec = g.apply(xs);

      // Zip together
      bs_vec.into_iter().zip(cs_vec).collect()
    })
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
    let f = VecDeque::<i32>::arrow(|x: i32| x * 2);
    let result = f.apply(VecDeque::from(vec![1, 2, 3]));
    assert_eq!(result, VecDeque::from(vec![2, 4, 6]));
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let xs = VecDeque::from(vec![1, 2, 3]);
    
    let f_arrow = VecDeque::<i32>::arrow(|x: i32| x * 2);
    let f_arr = VecDeque::<i32>::arr(|x: &i32| x * 2);
    
    let result_arrow = f_arrow.apply(xs.clone());
    let result_arr = f_arr.apply(xs);
    
    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = VecDeque::<i32>::arrow(|x: i32| x * 2);
    let g = VecDeque::<i32>::arrow(|x: i32| x + 1);
    
    let split_fn = VecDeque::<i32>::split::<i32, i32, i32, i32>(f, g);
    let input = VecDeque::from(vec![(1, 10), (2, 20), (3, 30)]);
    let result = split_fn.apply(input);
    
    assert_eq!(result, VecDeque::from(vec![(2, 11), (4, 21), (6, 31)]));
  }

  #[test]
  fn test_fanout() {
    let f = VecDeque::<i32>::arrow(|x: i32| x * 2);
    let g = VecDeque::<i32>::arrow(|x: i32| x + 1);
    
    let fanout_fn = VecDeque::<i32>::fanout(f, g);
    let input = VecDeque::from(vec![1, 2, 3]);
    let result = fanout_fn.apply(input);
    
    assert_eq!(result, VecDeque::from(vec![(2, 2), (4, 3), (6, 4)]));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      xs in prop::collection::vec(any::<i32>(), 0..10)
    ) {
      let f = VecDeque::<i32>::arrow(|x: i32| x.saturating_mul(2));
      let g = VecDeque::<i32>::arrow(|x: i32| x.saturating_add(1));
      
      let split_fn = VecDeque::<i32>::split::<i32, i32, i32, i32>(f, g);
      let input: VecDeque<(i32, i32)> = xs.iter().map(|&x| (x, x.saturating_mul(10))).collect();
      let result = split_fn.apply(input.clone());
      
      // Check that the structure is preserved
      assert_eq!(result.len(), input.len());
      
      // Check that the transformation is correct
      for (i, (x, y)) in input.iter().enumerate() {
        let (expected_x, expected_y) = result[i];
        assert_eq!(expected_x, x.saturating_mul(2));
        assert_eq!(expected_y, y.saturating_add(1));
      }
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      xs in prop::collection::vec(any::<i32>(), 0..10)
    ) {
      let f = VecDeque::<i32>::arrow(|x: i32| x.saturating_mul(2));
      let g = VecDeque::<i32>::arrow(|x: i32| x.saturating_add(1));
      
      let fanout_fn = VecDeque::<i32>::fanout(f, g);
      let result = fanout_fn.apply(VecDeque::from(xs.clone()));
      
      // Check that the structure is preserved
      assert_eq!(result.len(), xs.len());
      
      // Check that the transformation is correct
      for (i, x) in xs.iter().enumerate() {
        let (expected_x, expected_y) = result[i];
        assert_eq!(expected_x, x.saturating_mul(2));
        assert_eq!(expected_y, x.saturating_add(1));
      }
    }
  }
} 