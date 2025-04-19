use crate::traits::semigroup::Semigroup;

impl Semigroup for char {
  fn combine(self, other: Self) -> Self {
    // For char, we can't truly concatenate, so we'll return the second char
    // This maintains associativity while providing a meaningful operation
    other
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_chars() {
    // Basic test - combining chars returns the second char
    assert_eq!('a'.combine('b'), 'b');
    assert_eq!('1'.combine('2'), '2');
    assert_eq!('!'.combine('?'), '?');
  }

  #[test]
  fn test_associativity() {
    // Test associativity manually
    let a = 'a';
    let b = 'b';
    let c = 'c';

    let left = a.combine(b).combine(c);
    let right = a.combine(b.combine(c));

    assert_eq!(left, right);
    assert_eq!(left, 'c'); // Both should be 'c'
  }

  #[test]
  fn test_with_unicode() {
    // Test with Unicode characters
    assert_eq!('α'.combine('β'), 'β');
    assert_eq!('汉'.combine('字'), '字');
    assert_eq!('🦀'.combine('🔄'), '🔄');
  }

  // Property-based test for associativity
  proptest_semigroup_associativity!(prop_char_associativity, char, any::<char>());

  proptest! {
    #[test]
    fn prop_combine_returns_second(a in any::<char>(), b in any::<char>()) {
      prop_assert_eq!(a.combine(b), b);
    }

    #[test]
    fn prop_combine_multiple(a in any::<char>(), b in any::<char>(), c in any::<char>()) {
      // Combining multiple characters should always result in the last one
      prop_assert_eq!(a.combine(b).combine(c), c);
      prop_assert_eq!(a.combine(b.combine(c)), c);
    }
  }
}
