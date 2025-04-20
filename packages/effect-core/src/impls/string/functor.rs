use crate::impls::string::category::CharCategory;
use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

impl Functor<char> for CharCategory {
  type HigherSelf<U: CloneableThreadSafe> = Vec<U>;

  fn map<U, F>(self, _f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a char) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    // This is a dummy implementation since CharCategory is just a marker type
    // We don't actually have characters in it to map over
    Vec::new()
  }
}

// Create an extension trait for String that uses the CharCategory
pub trait StringFunctorExt {
  fn map_chars<U, F>(&self, f: F) -> Vec<U>
  where
    F: for<'a> FnMut(&'a char) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe;
}

impl StringFunctorExt for String {
  fn map_chars<U, F>(&self, mut f: F) -> Vec<U>
  where
    F: for<'a> FnMut(&'a char) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    self.chars().map(|c| f(&c)).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_identity_law() {
    let s = "hello".to_string();
    let id = |c: &char| *c;
    let result: Vec<char> = s.map_chars(id);

    // Identity law: map(id) should return the original characters
    assert_eq!(result, vec!['h', 'e', 'l', 'l', 'o']);
  }

  #[test]
  fn test_composition_law() {
    let s = "abc".to_string();

    // Function 1: Convert to uppercase
    let f = |c: &char| c.to_uppercase().to_string();

    // Function 2: Get the length
    let g = |s: &String| s.len();

    // Apply f then g
    let result1: Vec<usize> = s
      .clone()
      .map_chars(f.clone())
      .into_iter()
      .map(|s| g(&s))
      .collect();

    // Apply composition directly
    let h = move |c: &char| g(&f(c));
    let result2: Vec<usize> = s.map_chars(h);

    // Composition law: map(f).map(g) == map(g ∘ f)
    assert_eq!(result1, result2);
  }

  #[test]
  fn test_map_with_different_types() {
    // Map to ASCII values
    let s = "hello".to_string();
    let result: Vec<u32> = s.map_chars(|c| *c as u32);
    assert_eq!(result, vec![104, 101, 108, 108, 111]);

    // Map to boolean (is_lowercase)
    let s = "Hello".to_string();
    let result: Vec<bool> = s.map_chars(|c| c.is_lowercase());
    assert_eq!(result, vec![false, true, true, true, true]);

    // Map to Option
    let s = "12a45".to_string();
    let result: Vec<Option<u32>> = s.map_chars(|c| c.to_digit(10));
    assert_eq!(result, vec![Some(1), Some(2), None, Some(4), Some(5)]);
  }

  proptest! {
    #[test]
    fn prop_identity_preserves_content(s in "\\PC*") {
      let s = s.to_string();
      let id = |c: &char| *c;
      let result: Vec<char> = s.map_chars(id);
      let original_chars: Vec<char> = s.chars().collect();

      prop_assert_eq!(result, original_chars);
    }

    #[test]
    fn prop_composition_law(s in "\\PC*") {
      let s = s.to_string();

      // Two simple functions
      let f = |c: &char| c.to_uppercase().to_string();
      let g = |s: &String| s.len();

      // Apply f then g (with map_chars and then manually mapping g)
      let result1: Vec<usize> = s.clone().map_chars(f.clone()).into_iter().map(|s| g(&s)).collect();

      // Apply composition directly
      let h = move |c: &char| g(&f(c));
      let result2: Vec<usize> = s.map_chars(h);

      // Composition law: map(f).map(g) == map(g ∘ f)
      prop_assert_eq!(result1, result2);
    }
  }
}
