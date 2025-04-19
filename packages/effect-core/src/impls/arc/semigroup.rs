use crate::traits::semigroup::Semigroup;
use std::sync::Arc;

impl<T> Semigroup for Arc<T>
where
  T: Semigroup + Clone,
{
  fn combine(self, other: Self) -> Self {
    // Get clone of inner values, combine them, then wrap in a new Arc
    let self_inner = T::clone(&*self);
    let other_inner = T::clone(&*other);
    Arc::new(self_inner.combine(other_inner))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_basic() {
    // Test with i32, which has a Semigroup implementation
    let a = Arc::new(5);
    let b = Arc::new(10);
    let result = a.combine(b);
    assert_eq!(*result, 15);
  }

  #[test]
  fn test_combine_strings() {
    // Test with String, which has a Semigroup implementation
    let a = Arc::new(String::from("Hello, "));
    let b = Arc::new(String::from("world!"));
    let result = a.combine(b);
    assert_eq!(*result, "Hello, world!");
  }

  #[test]
  fn test_associativity() {
    // Test associativity manually
    let a = Arc::new(1);
    let b = Arc::new(2);
    let c = Arc::new(3);

    let left = a.clone().combine(b.clone()).combine(c.clone());
    let right = a.combine(b.combine(c));

    assert_eq!(*left, *right);
    assert_eq!(*left, 6); // 1 + 2 + 3 = 6
  }

  // Property-based tests for associativity with various types
  proptest_semigroup_associativity!(
    prop_arc_i32_associativity,
    Arc<i32>,
    any::<i32>().prop_map(Arc::new)
  );

  proptest_semigroup_associativity!(
    prop_arc_string_associativity,
    Arc<String>,
    any::<String>().prop_map(|s| Arc::new(s.chars().take(10).collect::<String>()))
  );

  proptest! {
    #[test]
    fn prop_arc_preserves_inner_value(a in any::<i32>(), b in any::<i32>()) {
      let arc_a = Arc::new(a);
      let arc_b = Arc::new(b);
      let result = arc_a.combine(arc_b);

      // The combined arc value should equal the combination of the inner values
      // Use wrapping_add to match how the i32 Semigroup is implemented
      prop_assert_eq!(*result, a.wrapping_add(b));
    }

    #[test]
    fn prop_arc_string_preserves_inner_value(
      a in any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
      b in any::<String>().prop_map(|s| s.chars().take(5).collect::<String>())
    ) {
      let arc_a = Arc::new(a.clone());
      let arc_b = Arc::new(b.clone());
      let result = arc_a.combine(arc_b);

      // The combined arc value should equal the combination of the inner values
      let expected = a + &b;
      prop_assert_eq!(result.as_ref(), &expected);
    }
  }
}
