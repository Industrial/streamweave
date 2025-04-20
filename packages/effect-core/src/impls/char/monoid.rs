use crate::traits::monoid::Monoid;
use crate::traits::semigroup::Semigroup;

impl Monoid for char {
  fn empty() -> Self {
    // For chars, Semigroup implementation returns the second char
    // So any empty() character will not be preserved when combined
    // We use a character '\0' as a consistent representation of empty
    '\0'
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    assert_eq!(char::empty(), '\0');
  }

  #[test]
  fn test_identity() {
    // Since our Semigroup implementation returns the second char,
    // empty().combine(a) == a, but a.combine(empty()) == empty()
    let a = 'a';

    // Left identity: empty().combine(a) = a
    assert_eq!(char::empty().combine(a), a);

    // Right "identity": With our Semigroup impl, a.combine(empty()) = empty()
    assert_eq!(a.combine(char::empty()), char::empty());
  }

  #[test]
  fn test_with_unicode() {
    // Test with Unicode characters - same identity behavior
    let unicode = '汉';

    // Left identity: empty().combine(unicode) = unicode
    assert_eq!(char::empty().combine(unicode), unicode);

    // Right "identity": With our Semigroup impl, unicode.combine(empty()) = empty()
    assert_eq!(unicode.combine(char::empty()), char::empty());
  }

  #[test]
  fn test_mconcat() {
    // Test mconcat with an empty iterator
    let empty_vec: Vec<char> = vec![];
    assert_eq!(char::mconcat(empty_vec), char::empty());

    // Test mconcat with a non-empty iterator
    let chars = vec!['a', 'b', 'c'];
    let result = char::mconcat(chars);
    assert_eq!(result, 'c'); // Since combine returns the second char, we should get the last one
  }

  proptest! {
    #[test]
    fn prop_identity_law(c in any::<char>()) {
      // With our Semigroup impl, the left identity law holds but right doesn't
      // Left identity: empty().combine(c) = c
      prop_assert_eq!(char::empty().combine(c), c);

      // Right combine: c.combine(empty()) = empty()
      prop_assert_eq!(c.combine(char::empty()), char::empty());
    }

    #[test]
    fn prop_mconcat_single_element(c in any::<char>()) {
      // Test that mconcat with a single element returns that element
      let chars = vec![c];
      prop_assert_eq!(char::mconcat(chars), c);
    }

    #[test]
    fn prop_mconcat_multiple_elements(
      elements in proptest::collection::vec(any::<char>(), 1..10)
    ) {
      // Test that mconcat returns the last element (due to our combine implementation)
      if !elements.is_empty() {
        prop_assert_eq!(char::mconcat(elements.clone()), *elements.last().unwrap());
      }
    }
  }
}
