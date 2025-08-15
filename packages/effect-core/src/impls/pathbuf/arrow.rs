use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;
use super::category::PathBufFn;
use std::path::PathBuf;

impl Arrow<PathBuf, PathBuf> for PathBuf {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    PathBufFn::new(move |x| f(x))
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<PathBuf, PathBuf>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(PathBuf, C), (PathBuf, D)> {
    PathBufFn::new(move |(p, c)| (f.apply(p), g.apply(c)))
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<PathBuf, PathBuf>,
    g: Self::Morphism<PathBuf, C>,
  ) -> Self::Morphism<PathBuf, (PathBuf, C)> {
    PathBufFn::new(move |p: PathBuf| (f.apply(p.clone()), g.apply(p)))
  }
}

#[cfg(test)]
mod tests {
  use crate::traits::arrow::Arrow;
  use crate::traits::category::Category;
  use proptest::prelude::*;
  use std::path::PathBuf;

  #[test]
  fn test_arrow_creation() {
    let f = PathBuf::arrow(|x: PathBuf| {
      let mut new_path = x;
      new_path.set_extension("txt");
      new_path
    });
    let input = PathBuf::from("/test/path");
    let result = f.apply(input);
    assert_eq!(result, PathBuf::from("/test/path.txt"));
  }

  #[test]
  fn test_arrow_laws() {
    // Test that arrow() creates the same result as arr() for compatible functions
    let input = PathBuf::from("/test/path");
    
    let f_arrow = PathBuf::arrow(|x: PathBuf| {
      let mut new_path = x;
      new_path.set_extension("txt");
      new_path
    });
    let f_arr = PathBuf::arr(|x: &PathBuf| {
      let mut new_path = x.clone();
      new_path.set_extension("txt");
      new_path
    });
    
    let result_arrow = f_arrow.apply(input.clone());
    let result_arr = f_arr.apply(input);
    
    assert_eq!(result_arrow, result_arr);
  }

  #[test]
  fn test_split() {
    let f = PathBuf::arrow(|x: PathBuf| {
      let mut new_path = x;
      new_path.set_extension("txt");
      new_path
    });
    let g = PathBuf::arrow(|x: i32| x + 1);
    
    let split_fn = PathBuf::split::<i32, i32, i32, i32>(f, g);
    let input = (PathBuf::from("/test/path"), 10);
    
    let result = split_fn.apply(input);
    assert_eq!(result, (PathBuf::from("/test/path.txt"), 11));
  }

  #[test]
  fn test_fanout() {
    let f = PathBuf::arrow(|x: PathBuf| {
      let mut new_path = x.clone();
      new_path.set_extension("txt");
      new_path
    });
    let g = PathBuf::arrow(|x: PathBuf| x.to_string_lossy().to_string());
    
    let fanout_fn = PathBuf::fanout(f, g);
    let input = PathBuf::from("/test/path");
    
    let result = fanout_fn.apply(input);
    assert_eq!(result, (PathBuf::from("/test/path.txt"), "/test/path".to_string()));
  }

  proptest! {
    #[test]
    fn prop_arrow_split_preserves_structure(
      p in any::<PathBuf>(),
      y in any::<i32>()
    ) {
      let f = PathBuf::arrow(|x: PathBuf| {
        let mut new_path = x;
        new_path.set_extension("txt");
        new_path
      });
      let g = PathBuf::arrow(|x: i32| x.saturating_add(1));
      
      let split_fn = PathBuf::split::<i32, i32, i32, i32>(f, g);
      let input = (p.clone(), y);
      
      let result = split_fn.apply(input);
      
      // Check that the transformation is correct
      let mut expected_path = p.clone();
      expected_path.set_extension("txt");
      assert_eq!(result.0, expected_path);
      assert_eq!(result.1, y.saturating_add(1));
    }

    #[test]
    fn prop_arrow_fanout_preserves_structure(
      p in any::<PathBuf>()
    ) {
      let f = PathBuf::arrow(|x: PathBuf| {
        let mut new_path = x.clone();
        new_path.set_extension("txt");
        new_path
      });
      let g = PathBuf::arrow(|x: PathBuf| x.to_string_lossy().to_string());
      
      let fanout_fn = PathBuf::fanout(f, g);
      let input = p.clone();
      
      let result = fanout_fn.apply(input);
      
      // Check that the transformation is correct
      let mut expected_path = p.clone();
      expected_path.set_extension("txt");
      assert_eq!(result.0, expected_path);
      assert_eq!(result.1, p.to_string_lossy().to_string());
    }
  }
} 