use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use super::category::CharFn;

impl Arrow<char, char> for char {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    CharFn::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<char, char>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(char, C), (char, D)> {
    CharFn::new(move |(c, x)| (f.apply(c), g.apply(x)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<char, char>,
    g: Self::Morphism<char, C>,
  ) -> Self::Morphism<char, (char, C)> {
    CharFn::new(move |c: char| (f.apply(c.clone()), g.apply(c)))
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use proptest::prelude::*;

  #[test]
  fn test_arrow_creation() {
    let f = char::arrow(|x: char| x.to_uppercase().next().unwrap_or(x));
    let input = 'a';
    let result = f.apply(input);
    assert_eq!(result, 'A');
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = 'a';
    
    let f_arrow = char::arrow(|x: char| x.to_uppercase().next().unwrap_or(x));
    let f_arr = char::arr(|x: &char| x.to_uppercase().next().unwrap_or(*x));
    
    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);
    
    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = char::arrow(|x: char| x.to_uppercase().next().unwrap_or(x));
    let g = char::arrow(|x: i32| x + 1);
    
    let split_fn = char::split::<i32, i32, i32, i32>(f, g);
    let input = ('a', 10);
    
    let result = split_fn.apply(input);
    assert_eq!(result, ('A', 11));
  }

  #[test]
  fn test_fanout() {
    let f = char::arrow(|x: char| x.to_uppercase().next().unwrap_or(x));
    let g = char::arrow(|x: char| x.to_lowercase().next().unwrap_or(x));
    
    let fanout_fn = char::fanout(f, g);
    let input = 'A';
    
    let result = fanout_fn.apply(input);
    assert_eq!(result, ('A', 'a'));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      c in any::<char>(),
      y in any::<i32>()
    ) {
      let f = char::arrow(|x: char| x.to_uppercase().next().unwrap_or(x));
      let g = char::arrow(|x: i32| x.saturating_add(1));
      
      let split_fn = char::split::<i32, i32, i32, i32>(f, g);
      let input = (c, y);
      
      let result = split_fn.apply(input);
      
      // Check that the transformation is correct
      assert_eq!(result.0, c.to_uppercase().next().unwrap_or(c));
      assert_eq!(result.1, y.saturating_add(1));
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      c in any::<char>()
    ) {
      let f = char::arrow(|x: char| x.to_uppercase().next().unwrap_or(x));
      let g = char::arrow(|x: char| x.to_lowercase().next().unwrap_or(x));
      
      let fanout_fn = char::fanout(f, g);
      let input = c;
      
      let result = fanout_fn.apply(input);
      
      // Check that the transformation is correct
      assert_eq!(result.0, c.to_uppercase().next().unwrap_or(c));
      assert_eq!(result.1, c.to_lowercase().next().unwrap_or(c));
    }
  }
} 