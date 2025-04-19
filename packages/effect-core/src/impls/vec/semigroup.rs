use crate::traits::semigroup::Semigroup;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe> Semigroup for Vec<T> {
  fn combine(mut self, mut other: Self) -> Self {
    self.append(&mut other);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::collection::vec;
  use proptest::prelude::*;

  #[test]
  fn test_combine_empty_vectors() {
    let vec1: Vec<i32> = vec![];
    let vec2: Vec<i32> = vec![];

    let result = vec1.combine(vec2);
    assert_eq!(result, Vec::<i32>::new());
  }

  #[test]
  fn test_combine_with_empty() {
    let vec1 = vec![1, 2, 3];
    let vec2: Vec<i32> = vec![];

    assert_eq!(vec1.clone().combine(vec2.clone()), vec![1, 2, 3]);
    assert_eq!(vec2.combine(vec1), vec![1, 2, 3]);
  }

  #[test]
  fn test_combine_non_empty() {
    let vec1 = vec![1, 2, 3];
    let vec2 = vec![4, 5, 6];

    assert_eq!(vec1.combine(vec2), vec![1, 2, 3, 4, 5, 6]);
  }

  #[test]
  fn test_combine_preserves_order() {
    let vec1 = vec![1, 2, 3];
    let vec2 = vec![4, 5, 6];
    let vec3 = vec![7, 8, 9];

    // Combine them in different orders and verify associativity
    let result1 = vec1.clone().combine(vec2.clone()).combine(vec3.clone());
    let result2 = vec1.combine(vec2.combine(vec3));

    // Both should preserve the original order of elements
    assert_eq!(result1, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(result2, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
  }

  #[test]
  fn test_associativity_manual() {
    // Test with some predefined vectors
    let vec1 = vec![1, 2, 3];
    let vec2 = vec![4, 5];
    let vec3 = vec![6, 7, 8];

    test_associativity(vec1, vec2, vec3);
  }

  // Property-based tests with bounded inputs
  proptest_semigroup_associativity!(
    prop_associativity_i32,
    Vec<i32>,
    vec(
      any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      0..5
    )
  );

  proptest_semigroup_associativity!(
    prop_associativity_string,
    Vec<String>,
    vec(
      any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
      0..5
    )
  );

  proptest! {
    #[test]
    fn prop_combine_length(
      vec1 in vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      vec2 in vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let combined = vec1.clone().combine(vec2.clone());
      prop_assert_eq!(combined.len(), vec1.len() + vec2.len());
    }

    #[test]
    fn prop_combine_content(
      vec1 in vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5),
      vec2 in vec(any::<i32>().prop_filter("Value too large", |v| *v < 10000), 0..5)
    ) {
      let combined = vec1.clone().combine(vec2.clone());

      // All elements from the first vector should be at the beginning
      for (idx, val) in vec1.iter().enumerate() {
        prop_assert_eq!(&combined[idx], val);
      }

      // All elements from the second vector should be at the end
      for (idx, val) in vec2.iter().enumerate() {
        prop_assert_eq!(&combined[vec1.len() + idx], val);
      }
    }
  }
}
