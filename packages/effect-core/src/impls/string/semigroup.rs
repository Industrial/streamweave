use crate::traits::semigroup::Semigroup;

impl Semigroup for String {
  fn combine(mut self, other: Self) -> Self {
    self.push_str(&other);
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_empty_strings() {
    let s1 = String::new();
    let s2 = String::new();
    assert_eq!(s1.combine(s2), String::new());
  }

  #[test]
  fn test_combine_with_empty() {
    let s1 = "Hello".to_string();
    let s2 = String::new();

    assert_eq!(s1.clone().combine(s2.clone()), "Hello".to_string());
    assert_eq!(s2.combine(s1), "Hello".to_string());
  }

  #[test]
  fn test_combine_non_empty() {
    let s1 = "Hello ".to_string();
    let s2 = "World".to_string();
    assert_eq!(s1.combine(s2), "Hello World".to_string());
  }

  #[test]
  fn test_combine_preserves_order() {
    let s1 = "Hello".to_string();
    let s2 = " World".to_string();
    let s3 = "!".to_string();

    // Combine them in different orders and verify associativity
    let result1 = s1.clone().combine(s2.clone()).combine(s3.clone());
    let result2 = s1.clone().combine(s2.clone().combine(s3.clone()));

    // Both should preserve the original order of elements
    assert_eq!(result1, "Hello World!".to_string());
    assert_eq!(result2, "Hello World!".to_string());
  }

  // Property-based tests with bounded inputs
  proptest_semigroup_associativity!(
    prop_associativity_string,
    String,
    any::<String>().prop_map(|s| s.chars().take(10).collect::<String>())
  );

  proptest! {
    #[test]
    fn prop_combine_length(
      s1 in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>()),
      s2 in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>())
    ) {
      let combined = s1.clone().combine(s2.clone());
      prop_assert_eq!(combined.len(), s1.len() + s2.len());
    }

    #[test]
    fn prop_combine_contains_substrings(
      s1 in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>()),
      s2 in any::<String>().prop_map(|s| s.chars().take(10).collect::<String>())
    ) {
      let combined = s1.clone().combine(s2.clone());

      // Combined string should start with s1
      prop_assert!(combined.starts_with(&s1));

      // Combined string should end with s2
      prop_assert!(combined.ends_with(&s2));
    }
  }
}
