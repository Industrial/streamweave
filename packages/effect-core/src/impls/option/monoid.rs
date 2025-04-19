use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Monoid for Option<T> {
  fn empty() -> Self {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  // Basic empty() test
  #[test]
  fn test_empty() {
    let empty_option: Option<i32> = Option::empty();
    assert_eq!(empty_option, None);

    let empty_option: Option<String> = Option::empty();
    assert_eq!(empty_option, None);
  }

  // Test left identity: empty.combine(x) = x
  #[test]
  fn test_left_identity() {
    // With None
    let none: Option<i32> = None;
    assert_eq!(Option::<i32>::empty().combine(none.clone()), none);

    // With Some
    let some = Some(42);
    assert_eq!(Option::<i32>::empty().combine(some.clone()), some);

    // With String
    let some_string = Some("hello".to_string());
    assert_eq!(
      Option::<String>::empty().combine(some_string.clone()),
      some_string
    );
  }

  // Test right identity: x.combine(empty) = x
  #[test]
  fn test_right_identity() {
    // With None
    let none: Option<i32> = None;
    assert_eq!(none.clone().combine(Option::<i32>::empty()), none);

    // With Some
    let some = Some(42);
    assert_eq!(some.clone().combine(Option::<i32>::empty()), some);

    // With String
    let some_string = Some("hello".to_string());
    assert_eq!(
      some_string.clone().combine(Option::<String>::empty()),
      some_string
    );
  }

  // Test mconcat functionality
  #[test]
  fn test_mconcat() {
    // Empty vector
    let empty_vec: Vec<Option<i32>> = vec![];
    assert_eq!(Option::mconcat(empty_vec), None);

    // Vector with all None
    let all_none = vec![None, None, None];
    assert_eq!(Option::<i32>::mconcat(all_none), None);

    // Vector with some None and some Some
    let mixed = vec![None, Some(1), None, Some(2), None];
    assert_eq!(Option::mconcat(mixed), Some(1)); // First Some wins in our implementation

    // Vector with all Some
    let all_some = vec![Some(3), Some(4), Some(5)];
    assert_eq!(Option::mconcat(all_some), Some(3)); // First Some wins
  }

  // Test that mconcat is equivalent to folding with combine
  #[test]
  fn test_mconcat_equivalence() {
    let options = vec![Some(1), None, Some(2), None, Some(3)];

    let mconcat_result = Option::mconcat(options.clone());
    let fold_result = options
      .into_iter()
      .fold(Option::empty(), |acc, x| acc.combine(x));

    assert_eq!(mconcat_result, fold_result);
  }

  // Property-based tests
  proptest! {
    // Test that empty is a left identity
    #[test]
    fn prop_left_identity(x in prop::option::of(1..100i32)) {
      prop_assert_eq!(Option::<i32>::empty().combine(x.clone()), x);
    }

    // Test that empty is a right identity
    #[test]
    fn prop_right_identity(x in prop::option::of(1..100i32)) {
      prop_assert_eq!(x.clone().combine(Option::<i32>::empty()), x);
    }

    // Test that mconcat is equivalent to fold
    #[test]
    fn prop_mconcat_equivalent_to_fold(xs in prop::collection::vec(prop::option::of(1..100i32), 0..10)) {
      let mconcat_result = Option::mconcat(xs.clone());
      let fold_result = xs.into_iter().fold(Option::empty(), |acc, x| acc.combine(x));
      prop_assert_eq!(mconcat_result, fold_result);
    }

    // Test for commutativity with empty
    #[test]
    fn prop_empty_commutativity(x in prop::option::of(1..100i32)) {
      prop_assert_eq!(
        Option::<i32>::empty().combine(x.clone()),
        x.clone().combine(Option::<i32>::empty())
      );
    }
  }
}
