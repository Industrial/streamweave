use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::path::PathBuf;
use std::sync::Arc;

/// A cloneable function wrapper for PathBuf
#[derive(Clone)]
pub struct PathBufFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> PathBufFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    PathBufFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

impl Category<PathBuf, PathBuf> for PathBuf {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = PathBufFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    PathBufFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    PathBufFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    PathBufFn::new(move |x| f(&x))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    PathBufFn::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    PathBufFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for PathBuf
  fn to_string(p: &PathBuf) -> String {
    p.to_string_lossy().to_string()
  }

  fn add_extension(p: &PathBuf) -> PathBuf {
    let mut new_path = p.clone();
    new_path.set_extension("txt");
    new_path
  }

  #[test]
  fn test_identity_law() {
    let p = PathBuf::from("/test/path");
    let id = <PathBuf as Category<PathBuf, PathBuf>>::id();

    assert_eq!(id.apply(p.clone()), p);
  }

  #[test]
  fn test_composition_law() {
    let p = PathBuf::from("/test/path");
    let f = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);
    let g = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);

    let f_then_g = <PathBuf as Category<PathBuf, PathBuf>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(p.clone()));

    assert_eq!(f_then_g.apply(p), expected);
  }

  #[test]
  fn test_arr_law() {
    let p = PathBuf::from("/test/path");
    let f = |x: &PathBuf| {
      let mut new_path = x.clone();
      new_path.set_extension("txt");
      new_path
    };
    let arr_f = <PathBuf as Category<PathBuf, PathBuf>>::arr(f);

    let result = arr_f.apply(p);
    let expected = PathBuf::from("/test/path.txt");

    assert_eq!(result, expected);
  }

  #[test]
  fn test_first() {
    let pair = (PathBuf::from("/test/path"), 5);
    let f = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);
    let first_f = <PathBuf as Category<PathBuf, PathBuf>>::first(f);

    let result = first_f.apply(pair);
    let expected_path = PathBuf::from("/test/path.txt");
    assert_eq!(result, (expected_path, 5));
  }

  #[test]
  fn test_second() {
    let pair = (5, PathBuf::from("/test/path"));
    let f = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);
    let second_f = <PathBuf as Category<PathBuf, PathBuf>>::second(f);

    let result = second_f.apply(pair);
    let expected_path = PathBuf::from("/test/path.txt");
    assert_eq!(result, (5, expected_path));
  }

  proptest! {
    #[test]
    fn prop_identity_preserves_structure(
      p in any::<PathBuf>()
    ) {
      let id = <PathBuf as Category<PathBuf, PathBuf>>::id();
      let result = id.apply(p.clone());

      assert_eq!(result, p);
    }

    #[test]
    fn prop_composition_preserves_structure(
      p in any::<PathBuf>()
    ) {
      let f = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);
      let g = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);

      let f_then_g = <PathBuf as Category<PathBuf, PathBuf>>::compose(f, g);
      let result = f_then_g.apply(p.clone());

      // Check that the transformation is correct
      let mut expected = p.clone();
      expected.set_extension("txt");
      expected.set_extension("txt"); // Apply twice
      assert_eq!(result, expected);
    }

    #[test]
    fn prop_arr_preserves_structure(
      p in any::<PathBuf>()
    ) {
      let f = |x: &PathBuf| {
        let mut new_path = x.clone();
        new_path.set_extension("txt");
        new_path
      };
      let arr_f = <PathBuf as Category<PathBuf, PathBuf>>::arr(f);

      let result = arr_f.apply(p.clone());

      // Check that the transformation is correct
      let mut expected = p.clone();
      expected.set_extension("txt");
      assert_eq!(result, expected);
    }

    #[test]
    fn prop_first_preserves_structure(
      p in any::<PathBuf>(),
      c in any::<i32>()
    ) {
      let f = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);
      let first_f = <PathBuf as Category<PathBuf, PathBuf>>::first(f);

      let input = (p.clone(), c);
      let result = first_f.apply(input);

      // Check that the transformation is correct
      let mut expected_path = p.clone();
      expected_path.set_extension("txt");
      assert_eq!(result.0, expected_path);
      assert_eq!(result.1, c);
    }

    #[test]
    fn prop_second_preserves_structure(
      c in any::<i32>(),
      p in any::<PathBuf>()
    ) {
      let f = <PathBuf as Category<PathBuf, PathBuf>>::arr(add_extension);
      let second_f = <PathBuf as Category<PathBuf, PathBuf>>::second(f);

      let input = (c, p.clone());
      let result = second_f.apply(input);

      // Check that the transformation is correct
      let mut expected_path = p.clone();
      expected_path.set_extension("txt");
      assert_eq!(result.0, c);
      assert_eq!(result.1, expected_path);
    }
  }
}
