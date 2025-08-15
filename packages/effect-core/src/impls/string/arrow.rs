use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use super::category::StringFn;

impl Arrow<String, String> for String {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    StringFn::new(move |x| f(x))
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
    StringFn::new(move |(s, c)| (f.apply(s), g.apply(c)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<String, String>,
    g: Self::Morphism<String, C>,
  ) -> Self::Morphism<String, (String, C)> {
    StringFn::new(move |s: String| (f.apply(s.clone()), g.apply(s)))
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use crate::impls::string::category::CharFn;

  #[test]
  fn test_arrow_creation() {
    let f = String::arrow(|x: i32| x * 2);
    let input = 5;
    let result = f.apply(input);
    assert_eq!(result, 10);
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = 5;
    
    let f_arrow = String::arrow(|x: i32| x * 2);
    let f_arr: CharFn<i32, i32> = <String as Category<char, char>>::arr::<i32, i32, _>(|x: &i32| x * 2);
    
    let result_arrow = f_arrow.apply(input);
    let result_arr = f_arr.apply(input);
    
    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = String::arrow(|x: String| x.to_uppercase());
    let g = String::arrow(|x: i32| x + 1);
    
    let split_fn = String::split::<i32, i32, i32, i32>(f, g);
    let input = ("hello".to_string(), 10);
    
    let result = split_fn.apply(input);
    assert_eq!(result, ("HELLO".to_string(), 11));
  }

  #[test]
  fn test_fanout() {
    let f = String::arrow(|x: String| x.to_uppercase());
    let g = String::arrow(|x: String| x.len());
    
    let fanout_fn = String::fanout(f, g);
    let input = "hello".to_string();
    
    let result = fanout_fn.apply(input);
    assert_eq!(result, ("HELLO".to_string(), 5));
  }

  // Additional tests for arrow properties
  #[test]
  fn test_arrow_split_preserves_structure() {
    let s = "hello".to_string();
    let y = 42;
    let f = String::arrow(|x: String| x.to_uppercase());
    let g = String::arrow(|x: i32| x.saturating_add(1));
    
    let split_fn = String::split::<i32, i32, i32, i32>(f, g);
    let input = (s.clone(), y);
    
    let result = split_fn.apply(input);
    
    // Check that the transformation is correct
    assert_eq!(result.0, s.to_uppercase());
    assert_eq!(result.1, y.saturating_add(1));
  }

  #[test]
  fn test_arrow_fanout_preserves_structure() {
    let s = "hello".to_string();
    let f = String::arrow(|x: String| x.to_uppercase());
    let g = String::arrow(|x: String| x.len());
    
    let fanout_fn = String::fanout(f, g);
    let input = s.clone();
    
    let result = fanout_fn.apply(input);
    
    // Check that the transformation is correct
    assert_eq!(result.0, s.to_uppercase());
    assert_eq!(result.1, s.len());
  }
} 