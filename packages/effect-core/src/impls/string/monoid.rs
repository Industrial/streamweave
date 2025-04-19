use crate::traits::monoid::Monoid;

impl Monoid for String {
  fn empty() -> Self {
    String::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_string = String::empty();
    assert_eq!(empty_string, "");
    assert_eq!(empty_string.len(), 0);
  }

  #[test]
  fn test_left_identity() {
    // empty().combine(a) = a
    let a = "Hello".to_string();
    let empty = String::empty();
    assert_eq!(empty.combine(a.clone()), a);
  }

  #[test]
  fn test_right_identity() {
    // a.combine(empty()) = a
    let a = "Hello".to_string();
    let empty = String::empty();
    assert_eq!(a.clone().combine(empty), a);
  }

  #[test]
  fn test_combine_with_empty() {
    // Test combining with empty() for different strings
    let a = "Hello World".to_string();

    // Create a new empty instance for each use
    assert_eq!(a.clone().combine(String::empty()), a);
    assert_eq!(String::empty().combine(a.clone()), a);

    let b = "123".to_string();
    assert_eq!(b.clone().combine(String::empty()), b);
    assert_eq!(String::empty().combine(b.clone()), b);
  }

  // Property-based tests
  proptest! {
    // Test left identity: empty().combine(a) = a
    #[test]
    fn prop_left_identity(a in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>())) {
      prop_assert_eq!(String::empty().combine(a.clone()), a);
    }

    // Test right identity: a.combine(empty()) = a
    #[test]
    fn prop_right_identity(a in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>())) {
      prop_assert_eq!(a.clone().combine(String::empty()), a);
    }

    // Test that mconcat on a list of strings is equivalent to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      strings in proptest::collection::vec(
        any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
        0..5
      )
    ) {
      // Create a clone to use with fold since we consume the original in mconcat
      let strings_clone = strings.clone();

      let folded = strings_clone.into_iter().fold(String::empty(), |acc, s| acc.combine(s));
      let mconcated = Monoid::mconcat(strings);

      prop_assert_eq!(folded, mconcated);
    }
  }
}
