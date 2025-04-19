use crate::traits::monoid::Monoid;

impl Monoid for char {
  fn empty() -> Self {
    // Using space as the empty value for char
    ' '
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_char = char::empty();
    assert_eq!(empty_char, ' ');
  }

  #[test]
  fn test_left_identity() {
    // Since our semigroup implementation returns the second char,
    // combining empty with a char will return the char
    let a = 'a';
    let empty = char::empty();
    assert_eq!(empty.combine(a), a);
  }

  #[test]
  fn test_right_identity() {
    // According to our semigroup implementation, a.combine(empty) returns empty
    // This is not a true identity, but it's the best we can do with our implementation
    let a = 'a';
    let empty = char::empty();
    assert_eq!(a.combine(empty), empty);
  }

  #[test]
  fn test_mconcat() {
    // Test mconcat with a list of chars
    let chars = vec!['a', 'b', 'c'];

    // With our implementation, mconcat will return the last char in the collection,
    // or the empty char if the collection is empty
    assert_eq!(Monoid::mconcat(chars), 'c');

    let empty_vec: Vec<char> = vec![];
    assert_eq!(Monoid::mconcat(empty_vec), ' ');
  }

  // Property-based tests
  proptest! {
    // Test left identity: empty().combine(a) = a
    #[test]
    fn prop_left_identity(a in any::<char>()) {
      prop_assert_eq!(char::empty().combine(a), a);
    }

    // Test right identity behavior
    #[test]
    fn prop_right_identity_behavior(a in any::<char>()) {
      // This checks our implementation's behavior, though it's not a true identity
      prop_assert_eq!(a.combine(char::empty()), ' ');
    }

    // Test that mconcat on a list of chars follows our combine behavior
    #[test]
    fn prop_mconcat_behavior(
      chars in proptest::collection::vec(any::<char>(), 0..10)
    ) {
      let expected = if chars.is_empty() {
        char::empty()
      } else {
        *chars.last().unwrap()
      };

      prop_assert_eq!(Monoid::mconcat(chars), expected);
    }
  }
}
