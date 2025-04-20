use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::sync::Arc;

/// A cloneable function wrapper for char
#[derive(Clone)]
pub struct CharFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> CharFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    CharFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

impl Category<char, char> for char {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = CharFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    CharFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    CharFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    CharFn::new(move |x| f(&x))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    CharFn::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    CharFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for chars
  fn to_uppercase(c: &char) -> char {
    c.to_uppercase().next().unwrap_or(*c)
  }

  fn to_lowercase(c: &char) -> char {
    c.to_lowercase().next().unwrap_or(*c)
  }

  #[test]
  fn test_identity_law() {
    let c = 'a';
    let id = <char as Category<char, char>>::arr(|&x| x);

    assert_eq!(id.apply(c), c);
  }

  #[test]
  fn test_composition_law() {
    let c = 'a';
    let f = <char as Category<char, char>>::arr(to_uppercase);
    let g = <char as Category<char, char>>::arr(to_lowercase);

    let f_then_g = <char as Category<char, char>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(c));

    assert_eq!(f_then_g.apply(c), expected);
    assert_eq!(f_then_g.apply('a'), 'a');
  }

  #[test]
  fn test_first() {
    let pair = ('a', 5);
    let f = <char as Category<char, char>>::arr(to_uppercase);
    let first_f = <char as Category<char, char>>::first(f);

    let result = first_f.apply(pair);
    assert_eq!(result, ('A', 5));
  }

  #[test]
  fn test_second() {
    let pair = (5, 'A');
    let f = <char as Category<char, char>>::arr(to_lowercase);
    let second_f = <char as Category<char, char>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result, (5, 'a'));
  }

  proptest! {
    #[test]
    fn prop_identity_law(c: char) {
      let id = <char as Category<char, char>>::arr(|&x| x);
      prop_assert_eq!(id.apply(c), c);
    }

    #[test]
    fn prop_composition_law(c: char) {
      // For any char, to_uppercase then to_lowercase should be the same as to_lowercase
      let upper = <char as Category<char, char>>::arr(to_uppercase);
      let lower = <char as Category<char, char>>::arr(to_lowercase);

      let composed = <char as Category<char, char>>::compose(upper.clone(), lower.clone());
      let composed_result = composed.apply(c);

      // For most chars, this should equal lowercase(c)
      // For special cases, just ensure the composition works
      let expected = lower.apply(upper.apply(c));
      prop_assert_eq!(composed_result, expected);
    }
  }
}
