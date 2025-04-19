use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;
use std::collections::VecDeque;

impl<T> Semigroup for VecDeque<T>
where
  T: CloneableThreadSafe,
{
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use proptest::prelude::*;

  // Helper function to convert a Vec to a VecDeque
  fn to_vecdeque<T: Clone>(v: Vec<T>) -> VecDeque<T> {
    v.into_iter().collect()
  }

  #[test]
  fn test_combine_empty_vecdeques() {
    let vd1: VecDeque<i32> = VecDeque::new();
    let vd2: VecDeque<i32> = VecDeque::new();

    let result = vd1.combine(vd2);
    assert_eq!(result, VecDeque::<i32>::new());
  }

  #[test]
  fn test_combine_with_empty() {
    let vd1 = to_vecdeque(vec![1, 2, 3]);
    let vd2: VecDeque<i32> = VecDeque::new();

    assert_eq!(vd1.clone().combine(vd2.clone()), to_vecdeque(vec![1, 2, 3]));
    assert_eq!(vd2.combine(vd1), to_vecdeque(vec![1, 2, 3]));
  }

  #[test]
  fn test_combine_non_empty() {
    let vd1 = to_vecdeque(vec![1, 2, 3]);
    let vd2 = to_vecdeque(vec![4, 5, 6]);

    assert_eq!(vd1.combine(vd2), to_vecdeque(vec![1, 2, 3, 4, 5, 6]));
  }

  #[test]
  fn test_combine_preserves_order() {
    let vd1 = to_vecdeque(vec![1, 2, 3]);
    let vd2 = to_vecdeque(vec![4, 5, 6]);
    let vd3 = to_vecdeque(vec![7, 8, 9]);

    // Combine them in different orders and verify associativity
    let result1 = vd1.clone().combine(vd2.clone()).combine(vd3.clone());
    let result2 = vd1.combine(vd2.combine(vd3));

    // Both should preserve the original order of elements
    assert_eq!(result1, to_vecdeque(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));
    assert_eq!(result2, to_vecdeque(vec![1, 2, 3, 4, 5, 6, 7, 8, 9]));
  }

  #[test]
  fn test_combine_with_strings() {
    let vd1 = to_vecdeque(vec!["Hello".to_string(), "world".to_string()]);
    let vd2 = to_vecdeque(vec!["!".to_string()]);

    assert_eq!(
      vd1.combine(vd2),
      to_vecdeque(vec![
        "Hello".to_string(),
        "world".to_string(),
        "!".to_string()
      ])
    );
  }

  // Property-based test for associativity
  proptest! {
    #[test]
    fn prop_associativity(
      a in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      b in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      c in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let vd_a = to_vecdeque(a);
      let vd_b = to_vecdeque(b);
      let vd_c = to_vecdeque(c);

      // Test that (a.combine(b)).combine(c) = a.combine(b.combine(c))
      let result1 = vd_a.clone().combine(vd_b.clone()).combine(vd_c.clone());
      let result2 = vd_a.clone().combine(vd_b.clone().combine(vd_c.clone()));

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_combine_length(
      a in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      b in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let vd_a = to_vecdeque(a.clone());
      let vd_b = to_vecdeque(b.clone());

      let combined = vd_a.combine(vd_b);

      // The length of the combined VecDeque should be the sum of the original lengths
      prop_assert_eq!(combined.len(), a.len() + b.len());
    }

    #[test]
    fn prop_combine_content(
      a in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      b in prop::collection::vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let vd_a = to_vecdeque(a.clone());
      let vd_b = to_vecdeque(b.clone());

      let combined = vd_a.combine(vd_b);
      let expected = to_vecdeque([a, b].concat());

      prop_assert_eq!(combined, expected);
    }
  }

  // Macro for property-based associativity test
  proptest_semigroup_associativity!(
    prop_associativity_int,
    VecDeque<i32>,
    prop::collection::vec(
      any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      0..5
    )
    .prop_map(|v| v.into_iter().collect::<VecDeque<i32>>())
  );

  proptest_semigroup_associativity!(
    prop_associativity_string,
    VecDeque<String>,
    prop::collection::vec(
      any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
      0..5
    )
    .prop_map(|v| v.into_iter().collect::<VecDeque<String>>())
  );
}
