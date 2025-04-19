use crate::traits::monoid::Monoid;

impl<A, B> Monoid for (A, B)
where
  A: Monoid,
  B: Monoid,
{
  fn empty() -> Self {
    (A::empty(), B::empty())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    // Test empty for tuples of integers
    let empty_tuple: (i32, i32) = Monoid::empty();
    assert_eq!(empty_tuple, (0, 0));

    // Test empty for tuples of booleans
    let empty_tuple: (bool, bool) = Monoid::empty();
    assert_eq!(empty_tuple, (false, false));

    // Test empty for tuples of strings
    let empty_tuple: (String, String) = Monoid::empty();
    assert_eq!(empty_tuple, (String::new(), String::new()));

    // Test empty for mixed types
    let empty_tuple: (i32, String) = Monoid::empty();
    assert_eq!(empty_tuple, (0, String::new()));
  }

  #[test]
  fn test_left_identity() {
    // Test left identity for tuples of integers
    let tuple = (1, 2);
    let empty = <(i32, i32)>::empty();
    assert_eq!(empty.combine(tuple), tuple);

    // Test left identity for tuples of strings
    let tuple = ("Hello".to_string(), "World".to_string());
    let empty = <(String, String)>::empty();
    assert_eq!(empty.combine(tuple.clone()), tuple);

    // Test left identity for mixed types
    let tuple = (1, "Hello".to_string());
    let empty = <(i32, String)>::empty();
    assert_eq!(empty.combine(tuple.clone()), tuple);
  }

  #[test]
  fn test_right_identity() {
    // Test right identity for tuples of integers
    let tuple = (1, 2);
    let empty = <(i32, i32)>::empty();
    assert_eq!(tuple.combine(empty), tuple);

    // Test right identity for tuples of strings
    let tuple = ("Hello".to_string(), "World".to_string());
    let empty = <(String, String)>::empty();
    assert_eq!(tuple.clone().combine(empty), tuple);

    // Test right identity for mixed types
    let tuple = (1, "Hello".to_string());
    let empty = <(i32, String)>::empty();
    assert_eq!(tuple.clone().combine(empty), tuple);
  }

  #[test]
  fn test_nested_tuples() {
    // Test empty for nested tuples
    let empty_tuple: ((i32, i32), (i32, i32)) = Monoid::empty();
    assert_eq!(empty_tuple, ((0, 0), (0, 0)));

    // Test identities for nested tuples
    let tuple = ((1, 2), (3, 4));
    let empty = <((i32, i32), (i32, i32))>::empty();

    assert_eq!(empty.combine(tuple), tuple);
    assert_eq!(tuple.combine(empty), tuple);
  }

  // Property-based tests
  proptest! {
    // Test left identity: empty().combine(a) = a
    #[test]
    fn prop_left_identity(
      a in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      b in any::<i32>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      let tuple = (a, b);
      let empty = <(i32, i32)>::empty();
      prop_assert_eq!(empty.combine(tuple), tuple);
    }

    // Test right identity: a.combine(empty()) = a
    #[test]
    fn prop_right_identity(
      a in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      b in any::<i32>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      let tuple = (a, b);
      let empty = <(i32, i32)>::empty();
      prop_assert_eq!(tuple.combine(empty), tuple);
    }

    // Test that mconcat is equivalent to folding with combine
    #[test]
    fn prop_mconcat_equivalent_to_fold(
      tuples in proptest::collection::vec(
        (any::<i32>().prop_filter("Value too large", |v| *v < 1000),
         any::<i32>().prop_filter("Value too large", |v| *v < 1000)),
        0..5
      )
    ) {
      // Create a clone for folding
      let tuples_clone = tuples.clone();

      let folded = tuples_clone.into_iter().fold(<(i32, i32)>::empty(), |acc, t| acc.combine(t));
      let mconcated = Monoid::mconcat(tuples);

      prop_assert_eq!(folded, mconcated);
    }

    // Test with mixed types
    #[test]
    fn prop_mixed_types(
      i in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      s in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>())
    ) {
      // Test left identity
      let tuple1 = (i, s.clone());
      let empty1 = <(i32, String)>::empty();
      prop_assert_eq!(empty1.combine(tuple1), (i, s.clone()));

      // Test right identity
      let tuple2 = (i, s.clone());  // Clone s here
      let empty2 = <(i32, String)>::empty();

      // Create a fresh expected tuple to avoid moved value issues
      let expected = (i, s);
      prop_assert_eq!(tuple2.combine(empty2), expected);
    }
  }
}
