use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Monoid for Vec<T> {
  fn empty() -> Self {
    Vec::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::collection::vec;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    let empty_vec: Vec<i32> = Monoid::empty();
    assert_eq!(empty_vec, Vec::<i32>::new());
    assert_eq!(empty_vec.len(), 0);

    let empty_vec: Vec<String> = Monoid::empty();
    assert_eq!(empty_vec, Vec::<String>::new());
    assert_eq!(empty_vec.len(), 0);
  }

  #[test]
  fn test_left_identity() {
    // empty().combine(a) = a
    let a = vec![1, 2, 3];
    let empty = Vec::<i32>::empty();
    assert_eq!(empty.combine(a.clone()), a);

    let a = vec!["Hello".to_string(), "World".to_string()];
    let empty = Vec::<String>::empty();
    assert_eq!(empty.combine(a.clone()), a);
  }

  #[test]
  fn test_right_identity() {
    // a.combine(empty()) = a
    let a = vec![1, 2, 3];
    let empty = Vec::<i32>::empty();
    assert_eq!(a.clone().combine(empty), a);

    let a = vec!["Hello".to_string(), "World".to_string()];
    let empty = Vec::<String>::empty();
    assert_eq!(a.clone().combine(empty), a);
  }

  #[test]
  fn test_combine_with_empty() {
    // Test combining with empty() for different types
    let a = vec![1, 2, 3];
    let empty = Vec::<i32>::empty();
    assert_eq!(a.clone().combine(empty.clone()), a);
    assert_eq!(empty.combine(a.clone()), a);

    let a = vec!["Hello".to_string(), "World".to_string()];
    let empty = Vec::<String>::empty();
    assert_eq!(a.clone().combine(empty.clone()), a);
    assert_eq!(empty.combine(a.clone()), a);
  }

  // Property-based tests
  proptest! {
    // Test left identity: empty().combine(a) = a
    #[test]
    fn prop_left_identity(a in vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)) {
      let empty = Vec::<i32>::empty();
      prop_assert_eq!(empty.combine(a.clone()), a);
    }

    // Test right identity: a.combine(empty()) = a
    #[test]
    fn prop_right_identity(a in vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)) {
      let empty = Vec::<i32>::empty();
      prop_assert_eq!(a.clone().combine(empty), a);
    }

    // Test with strings
    #[test]
    fn prop_identity_strings(
      a in vec(any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()), 0..5)
    ) {
      let empty = Vec::<String>::empty();

      // Test left identity
      prop_assert_eq!(empty.clone().combine(a.clone()), a.clone());

      // Test right identity
      prop_assert_eq!(a.clone().combine(empty), a);
    }

    // Test that mconcat on a list of vectors is equivalent to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      vecs in vec(
        vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..3),
        0..3
      )
    ) {
      // Create a clone for folding
      let vecs_clone = vecs.clone();

      let folded = vecs_clone.into_iter().fold(Vec::<i32>::empty(), |acc, v| acc.combine(v));
      let mconcated = Monoid::mconcat(vecs);

      prop_assert_eq!(folded, mconcated);
    }

    // Test that mconcat preserves the order of all elements
    #[test]
    fn prop_mconcat_preserves_order(
      vecs in vec(
        vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..3),
        1..4
      )
    ) {
      let mconcated = Monoid::mconcat(vecs.clone());

      // Calculate the expected result by flattening all vectors
      let mut expected = Vec::new();
      for v in vecs {
        expected.extend(v);
      }

      prop_assert_eq!(mconcated, expected);
    }
  }
}
