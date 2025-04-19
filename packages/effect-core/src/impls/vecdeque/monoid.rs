use crate::traits::monoid::Monoid;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::VecDeque;

impl<T> Monoid for VecDeque<T>
where
  T: CloneableThreadSafe,
{
  fn empty() -> Self {
    VecDeque::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  // Helper function to convert a Vec to a VecDeque
  fn to_vecdeque<T: Clone>(v: Vec<T>) -> VecDeque<T> {
    v.into_iter().collect()
  }

  #[test]
  fn test_empty() {
    let empty_vec: VecDeque<i32> = Monoid::empty();
    assert_eq!(empty_vec, VecDeque::<i32>::new());
    assert_eq!(empty_vec.len(), 0);

    let empty_vec: VecDeque<String> = Monoid::empty();
    assert_eq!(empty_vec, VecDeque::<String>::new());
    assert_eq!(empty_vec.len(), 0);
  }

  #[test]
  fn test_left_identity() {
    // empty().combine(a) = a
    let a = to_vecdeque(vec![1, 2, 3]);
    let empty = VecDeque::<i32>::empty();
    assert_eq!(empty.combine(a.clone()), a);

    let a = to_vecdeque(vec!["Hello".to_string(), "World".to_string()]);
    let empty = VecDeque::<String>::empty();
    assert_eq!(empty.combine(a.clone()), a);
  }

  #[test]
  fn test_right_identity() {
    // a.combine(empty()) = a
    let a = to_vecdeque(vec![1, 2, 3]);
    let empty = VecDeque::<i32>::empty();
    assert_eq!(a.clone().combine(empty), a);

    let a = to_vecdeque(vec!["Hello".to_string(), "World".to_string()]);
    let empty = VecDeque::<String>::empty();
    assert_eq!(a.clone().combine(empty), a);
  }

  #[test]
  fn test_combine_with_empty() {
    // Test combining with empty() for different types
    let a = to_vecdeque(vec![1, 2, 3]);
    let empty = VecDeque::<i32>::empty();
    assert_eq!(a.clone().combine(empty.clone()), a);
    assert_eq!(empty.combine(a.clone()), a);

    let a = to_vecdeque(vec!["Hello".to_string(), "World".to_string()]);
    let empty = VecDeque::<String>::empty();
    assert_eq!(a.clone().combine(empty.clone()), a);
    assert_eq!(empty.combine(a.clone()), a);
  }

  // Property-based tests
  proptest! {
    // Test left identity: empty().combine(a) = a
    #[test]
    fn prop_left_identity(
      a in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let vecdeque = to_vecdeque(a.clone());
      let empty = VecDeque::<i32>::empty();
      prop_assert_eq!(empty.combine(vecdeque.clone()), vecdeque);
    }

    // Test right identity: a.combine(empty()) = a
    #[test]
    fn prop_right_identity(
      a in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let vecdeque = to_vecdeque(a.clone());
      let empty = VecDeque::<i32>::empty();
      prop_assert_eq!(vecdeque.clone().combine(empty), vecdeque);
    }

    // Test with strings
    #[test]
    fn prop_identity_strings(
      a in prop::collection::vec(any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()), 0..5)
    ) {
      let vecdeque = to_vecdeque(a.clone());
      let empty = VecDeque::<String>::empty();

      // Test left identity
      prop_assert_eq!(empty.clone().combine(vecdeque.clone()), vecdeque.clone());

      // Test right identity
      prop_assert_eq!(vecdeque.clone().combine(empty), vecdeque);
    }

    // Test that mconcat on a list of VecDeques is equivalent to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      vecs in prop::collection::vec(
        prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..3),
        0..3
      )
    ) {
      // Convert all Vecs to VecDeques
      let vecdeques: Vec<VecDeque<i32>> = vecs.iter().map(|v| to_vecdeque(v.clone())).collect();

      // Create a clone for folding
      let vecdeques_clone = vecdeques.clone();

      let folded = vecdeques_clone.into_iter().fold(VecDeque::<i32>::empty(), |acc, v| acc.combine(v));
      let mconcated = Monoid::mconcat(vecdeques);

      prop_assert_eq!(folded, mconcated);
    }

    // Test that mconcat preserves the order of all elements
    #[test]
    fn prop_mconcat_preserves_order(
      vecs in prop::collection::vec(
        prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..3),
        1..4
      )
    ) {
      // Convert all Vecs to VecDeques
      let vecdeques: Vec<VecDeque<i32>> = vecs.iter().map(|v| to_vecdeque(v.clone())).collect();

      let mconcated = Monoid::mconcat(vecdeques.clone());

      // Calculate the expected result by flattening all vectors
      let mut expected = VecDeque::new();
      for v in &vecs {
        expected.extend(v);
      }

      prop_assert_eq!(mconcated, expected);
    }
  }
}
