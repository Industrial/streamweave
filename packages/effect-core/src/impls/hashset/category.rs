use crate::traits::category::Category;
use crate::types::threadsafe::CloneableThreadSafe;
use std::sync::Arc;

/// A morphism for HashSet<T> that represents transformations from one type to another
#[derive(Clone)]
pub struct HashSetFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> HashSetFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    HashSetFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

/// A proxy struct to implement Category for HashSet
#[derive(Clone, Copy)]
pub struct HashSetCategory;

impl<T, U> Category<T, U> for HashSetCategory
where
  T: CloneableThreadSafe,
  U: CloneableThreadSafe,
{
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = HashSetFn<A, B>;

  /// The identity morphism for HashSet - returns the input unchanged
  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    HashSetFn::new(|x| x)
  }

  /// Compose two morphisms f: A -> B and g: B -> C to get a morphism A -> C
  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    HashSetFn::new(move |x| g.apply(f.apply(x)))
  }

  /// Lift a regular function to a morphism
  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    HashSetFn::new(move |x| f(&x))
  }

  /// Create a morphism that applies f to the first component of a pair
  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    HashSetFn::new(move |(a, c)| (f.apply(a), c))
  }

  /// Create a morphism that applies f to the second component of a pair
  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    HashSetFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_identity_law() {
    let s = 5;
    let id = <HashSetCategory as Category<i32, i32>>::id();

    assert_eq!(id.apply(s), s);
  }

  #[test]
  fn test_composition_law() {
    let s = 5;

    let f = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x + 1);
    let g = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x * 2);

    let f_then_g = <HashSetCategory as Category<i32, i32>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(s));

    assert_eq!(f_then_g.apply(s), expected);
    assert_eq!(f_then_g.apply(3), 8); // (3+1)*2 = 8
  }

  #[test]
  fn test_arr_law() {
    let s = 5;
    let f = |x: &i32| x + 1;
    let arr_f = <HashSetCategory as Category<i32, i32>>::arr(f);

    let result = arr_f.apply(s);
    let expected = 6;

    assert_eq!(result, expected);
  }

  #[test]
  fn test_first() {
    let s = (1, "a");
    let f = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x + 1);
    let first_f = <HashSetCategory as Category<i32, i32>>::first(f);

    let result = first_f.apply(s);
    let expected = (2, "a");

    assert_eq!(result, expected);
  }

  #[test]
  fn test_second() {
    let s = ("a", 1);
    let f = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x + 1);
    let second_f = <HashSetCategory as Category<i32, i32>>::second(f);

    let result = second_f.apply(s);
    let expected = ("a", 2);

    assert_eq!(result, expected);
  }

  proptest! {
    #[test]
    fn prop_identity_preserves_structure(
      x in any::<i32>()
    ) {
      let id = <HashSetCategory as Category<i32, i32>>::id();
      let result = id.apply(x);

      assert_eq!(result, x);
    }

    #[test]
    fn prop_composition_preserves_structure(
      x in any::<i32>()
    ) {
      let f = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x.saturating_add(1));
      let g = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x.saturating_mul(2));

      let f_then_g = <HashSetCategory as Category<i32, i32>>::compose(f, g);
      let result = f_then_g.apply(x);

      // Check that the transformation is correct
      assert_eq!(result, (x.saturating_add(1)).saturating_mul(2));
    }

    #[test]
    fn prop_arr_preserves_structure(
      x in any::<i32>()
    ) {
      let f = |x: &i32| x.saturating_add(1);
      let arr_f = <HashSetCategory as Category<i32, i32>>::arr(f);

      let result = arr_f.apply(x);

      // Check that the transformation is correct
      assert_eq!(result, x.saturating_add(1));
    }

    #[test]
    fn prop_first_preserves_structure(
      x in any::<i32>(),
      c in any::<String>()
    ) {
      let f = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x.saturating_add(1));
      let first_f = <HashSetCategory as Category<i32, i32>>::first(f);

      let input = (x, c.clone());
      let result = first_f.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, x.saturating_add(1));
      assert_eq!(result.1, c);
    }

    #[test]
    fn prop_second_preserves_structure(
      c in any::<String>(),
      x in any::<i32>()
    ) {
      let f = <HashSetCategory as Category<i32, i32>>::arr(|x: &i32| x.saturating_add(1));
      let second_f = <HashSetCategory as Category<i32, i32>>::second(f);

      let input = (c.clone(), x);
      let result = second_f.apply(input);

      // Check that the transformation is correct
      assert_eq!(result.0, c);
      assert_eq!(result.1, x.saturating_add(1));
    }
  }
}
