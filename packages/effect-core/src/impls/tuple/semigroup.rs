use crate::traits::semigroup::Semigroup;

impl<A, B> Semigroup for (A, B)
where
  A: Semigroup,
  B: Semigroup,
{
  fn combine(self, other: Self) -> Self {
    (self.0.combine(other.0), self.1.combine(other.1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_tuples() {
    // Test with integers
    let tuple1 = (1, 2);
    let tuple2 = (3, 4);
    assert_eq!(tuple1.combine(tuple2), (4, 6));

    // Test with strings
    let tuple1 = ("Hello".to_string(), "World".to_string());
    let tuple2 = ("!".to_string(), "?".to_string());
    assert_eq!(
      tuple1.combine(tuple2),
      ("Hello!".to_string(), "World?".to_string())
    );

    // Test with mixed types
    let tuple1 = (1, "Hello".to_string());
    let tuple2 = (2, " World".to_string());
    assert_eq!(tuple1.combine(tuple2), (3, "Hello World".to_string()));
  }

  #[test]
  fn test_combine_nested_tuples() {
    // Test with nested tuples
    let tuple1 = ((1, 2), (3, 4));
    let tuple2 = ((5, 6), (7, 8));
    assert_eq!(tuple1.combine(tuple2), ((6, 8), (10, 12)));
  }

  #[test]
  fn test_associativity() {
    // Test associativity for tuples
    let a = (1, "Hello".to_string());
    let b = (2, " World".to_string());
    let c = (3, "!".to_string());

    let result1 = a.clone().combine(b.clone()).combine(c.clone());
    let result2 = a.clone().combine(b.clone().combine(c.clone()));

    assert_eq!(result1, result2);
    assert_eq!(result1, (6, "Hello World!".to_string()));
  }

  // Property-based tests
  proptest_semigroup_associativity!(
    prop_associativity_int_int,
    (i32, i32),
    (
      any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      any::<i32>().prop_filter("Value too large", |v| *v < 10000)
    )
  );

  proptest! {
    #[test]
    fn prop_component_wise_combine(
      a1 in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      a2 in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      b1 in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      b2 in any::<i32>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      let tuple1 = (a1, b1);
      let tuple2 = (a2, b2);

      // The result should be the component-wise combine of the tuples
      let expected = (a1.wrapping_add(a2), b1.wrapping_add(b2));
      prop_assert_eq!(tuple1.combine(tuple2), expected);
    }

    #[test]
    fn prop_nested_tuples(
      a1 in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      a2 in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      b1 in any::<i32>().prop_filter("Value too large", |v| *v < 10000),
      b2 in any::<i32>().prop_filter("Value too large", |v| *v < 10000)
    ) {
      let tuple1 = ((a1, b1), (a1, b1));
      let tuple2 = ((a2, b2), (a2, b2));

      // The result should be the nested component-wise combine
      let expected = (
        (a1.wrapping_add(a2), b1.wrapping_add(b2)),
        (a1.wrapping_add(a2), b1.wrapping_add(b2))
      );
      prop_assert_eq!(tuple1.combine(tuple2), expected);
    }
  }
}
