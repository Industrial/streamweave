use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::sync::Arc;

/// A cloneable function wrapper for String
#[derive(Clone)]
pub struct StringFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> StringFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    StringFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

impl Category<String, String> for String {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = StringFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    StringFn::new(|x| x)
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    StringFn::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    StringFn::new(move |x| f(&x))
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    StringFn::new(move |(a, c)| (f.apply(a), c))
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    StringFn::new(move |(c, a)| (c, f.apply(a)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for strings
  fn to_uppercase(s: &String) -> String {
    s.to_uppercase()
  }

  fn to_lowercase(s: &String) -> String {
    s.to_lowercase()
  }

  fn trim(s: &String) -> String {
    s.trim().to_string()
  }

  fn append_exclamation(s: &String) -> String {
    format!("{}!", s)
  }

  #[test]
  fn test_identity_law() {
    let s = String::from("test");
    let id = <String as Category<String, String>>::arr(|x: &String| x.clone());

    assert_eq!(id.apply(s.clone()), s);
  }

  #[test]
  fn test_composition_law() {
    let s = String::from("  Hello  ");
    let f = <String as Category<String, String>>::arr(trim);
    let g = <String as Category<String, String>>::arr(to_uppercase);

    let f_then_g = <String as Category<String, String>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(s.clone()));

    assert_eq!(f_then_g.apply(s), expected);
    assert_eq!(f_then_g.apply(String::from("  Hello  ")), "HELLO");
  }

  #[test]
  fn test_first() {
    let pair = (String::from("hello"), 5);
    let f = <String as Category<String, String>>::arr(to_uppercase);
    let first_f = <String as Category<String, String>>::first(f);

    let result = first_f.apply(pair);
    assert_eq!(result, (String::from("HELLO"), 5));
  }

  #[test]
  fn test_second() {
    let pair = (5, String::from("WORLD"));
    let f = <String as Category<String, String>>::arr(to_lowercase);
    let second_f = <String as Category<String, String>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result, (5, String::from("world")));
  }

  #[test]
  fn test_chain_operations() {
    let s = String::from("hello");
    let upper = <String as Category<String, String>>::arr(to_uppercase);
    let exclaim = <String as Category<String, String>>::arr(append_exclamation);

    let upper_then_exclaim = <String as Category<String, String>>::compose(upper, exclaim);
    let result = upper_then_exclaim.apply(s);

    assert_eq!(result, "HELLO!");
  }

  proptest! {
    #[test]
    fn prop_identity_law(s in "\\PC*") {
      let test_string = s.clone();
      let id = <String as Category<String, String>>::arr(|x: &String| x.clone());
      prop_assert_eq!(id.apply(test_string.clone()), test_string);
    }

    #[test]
    fn prop_composition_law(s in "\\PC*") {
      let test_string = s.clone();
      let trim_fn = <String as Category<String, String>>::arr(trim);
      let upper_fn = <String as Category<String, String>>::arr(to_uppercase);

      let composed = <String as Category<String, String>>::compose(trim_fn.clone(), upper_fn.clone());
      let composed_result = composed.apply(test_string.clone());

      let expected = upper_fn.apply(trim_fn.apply(test_string));
      prop_assert_eq!(composed_result, expected);
    }
  }
}
