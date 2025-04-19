use crate::traits::semigroup::Semigroup;

impl<T> Semigroup for Box<T>
where
  T: Semigroup,
{
  fn combine(self, other: Self) -> Self {
    // Unbox values, combine them using T's implementation, then box the result
    Box::new((*self).combine(*other))
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
    let a = Box::new(5);
    let b = Box::new(10);
    assert_eq!(*a.combine(b), 15);
  }

  #[test]
  fn test_combine_strings() {
    // Test with String, which has a Semigroup implementation
    let a = Box::new(String::from("Hello, "));
    let b = Box::new(String::from("world!"));
    assert_eq!(*a.combine(b), "Hello, world!");
  }

  #[test]
  fn test_associativity() {
    // Test associativity manually
    let a = Box::new(1);
    let b = Box::new(2);
    let c = Box::new(3);

    let left = a.clone().combine(b.clone()).combine(c.clone());
    let right = a.combine(b.combine(c));

    assert_eq!(*left, *right);
    assert_eq!(*left, 6); // 1 + 2 + 3 = 6
  }

  // Property-based tests for associativity with various types
  proptest_semigroup_associativity!(
    prop_boxed_i32_associativity,
    Box<i32>,
    any::<i32>().prop_map(Box::new)
  );

  proptest_semigroup_associativity!(
    prop_boxed_string_associativity,
    Box<String>,
    any::<String>().prop_map(|s| Box::new(s.chars().take(10).collect::<String>()))
  );

  proptest! {
    #[test]
    fn prop_boxed_preserves_inner_value(a in any::<i32>(), b in any::<i32>()) {
      let boxed_a = Box::new(a);
      let boxed_b = Box::new(b);

      // The combined boxed value should equal the combination of the inner values
      // Use wrapping_add to match how the i32 Semigroup is implemented
      prop_assert_eq!(*boxed_a.combine(boxed_b), a.wrapping_add(b));
    }

    #[test]
    fn prop_boxed_string_preserves_inner_value(
      a in any::<String>().prop_map(|s| s.chars().take(5).collect::<String>()),
      b in any::<String>().prop_map(|s| s.chars().take(5).collect::<String>())
    ) {
      let boxed_a = Box::new(a.clone());
      let boxed_b = Box::new(b.clone());

      // The combined boxed value should equal the combination of the inner values
      let expected = a + &b;
      prop_assert_eq!(*boxed_a.combine(boxed_b), expected);
    }
  }
}
