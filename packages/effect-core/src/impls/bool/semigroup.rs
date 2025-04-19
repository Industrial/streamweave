use crate::traits::semigroup::Semigroup;

impl Semigroup for bool {
  fn combine(self, other: Self) -> Self {
    self || other
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::prelude::*;

  #[test]
  fn test_combine_truth_table() {
    // Test all possible combinations
    assert_eq!(false.combine(false), false);
    assert_eq!(false.combine(true), true);
    assert_eq!(true.combine(false), true);
    assert_eq!(true.combine(true), true);
  }

  #[test]
  fn test_associativity_manual() {
    // Test all permutations of booleans for associativity
    test_associativity(false, false, false);
    test_associativity(false, false, true);
    test_associativity(false, true, false);
    test_associativity(false, true, true);
    test_associativity(true, false, false);
    test_associativity(true, false, true);
    test_associativity(true, true, false);
    test_associativity(true, true, true);
  }

  // Property-based test for associativity
  proptest_semigroup_associativity!(prop_associativity_bool, bool, any::<bool>());

  proptest! {
    #[test]
    fn prop_combine_is_or(a in any::<bool>(), b in any::<bool>()) {
      // Verify that combine behaves the same as the logical OR operation
      prop_assert_eq!(a.combine(b), a || b);
    }

    #[test]
    fn prop_combine_commutative(a in any::<bool>(), b in any::<bool>()) {
      // Boolean OR is commutative, so combine should be too
      prop_assert_eq!(a.combine(b), b.combine(a));
    }
  }
}
