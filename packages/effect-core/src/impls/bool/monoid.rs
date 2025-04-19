use crate::traits::monoid::Monoid;

impl Monoid for bool {
  fn empty() -> Self {
    false
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  #[test]
  fn test_empty() {
    assert!(!bool::empty());
  }

  #[test]
  fn test_empty_is_identity() {
    // Test that empty() is a left identity
    assert!(!bool::empty().combine(false));
    assert!(bool::empty().combine(true));

    // Test that empty() is a right identity
    assert!(!false.combine(bool::empty()));
    assert!(true.combine(bool::empty()));
  }

  #[test]
  fn test_mconcat_empty_iterator() {
    let bools: Vec<bool> = vec![];
    assert_eq!(bool::mconcat(bools), bool::empty());
  }

  #[test]
  fn test_mconcat_with_values() {
    // All false values should yield false
    let all_false = vec![false, false, false];
    assert!(!bool::mconcat(all_false));

    // Any true value should yield true due to OR operation
    let with_true = vec![false, true, false];
    assert!(bool::mconcat(with_true));

    let all_true = vec![true, true, true];
    assert!(bool::mconcat(all_true));
  }

  #[test]
  fn test_all_combine_permutations() {
    // Test all possible permutations of combine for bool (true, false)
    // true ⊕ true
    assert!(true.combine(true));
    // true ⊕ false
    assert!(true.combine(false));
    // false ⊕ true
    assert!(false.combine(true));
    // false ⊕ false
    assert!(!false.combine(false));
  }

  #[test]
  fn test_associativity() {
    // (true ⊕ true) ⊕ false = true ⊕ (true ⊕ false)
    assert_eq!(
      (true.combine(true)).combine(false),
      true.combine(true.combine(false))
    );

    // (true ⊕ false) ⊕ true = true ⊕ (false ⊕ true)
    assert_eq!(
      (true.combine(false)).combine(true),
      true.combine(false.combine(true))
    );

    // (false ⊕ true) ⊕ false = false ⊕ (true ⊕ false)
    assert_eq!(
      (false.combine(true)).combine(false),
      false.combine(true.combine(false))
    );

    // (false ⊕ false) ⊕ true = false ⊕ (false ⊕ true)
    assert_eq!(
      (false.combine(false)).combine(true),
      false.combine(false.combine(true))
    );

    // Test all possibilities with three operations
    // All possible permutations of three booleans (2^3 = 8 cases)
    let bools = [true, false];
    for a in bools.iter() {
      for b in bools.iter() {
        for c in bools.iter() {
          assert_eq!((a.combine(*b)).combine(*c), a.combine(b.combine(*c)));
        }
      }
    }
  }

  proptest! {
    #[test]
    fn prop_empty_is_left_identity(a in any::<bool>()) {
      prop_assert_eq!(bool::empty().combine(a), a);
    }

    #[test]
    fn prop_empty_is_right_identity(a in any::<bool>()) {
      prop_assert_eq!(a.combine(bool::empty()), a);
    }

    #[test]
    fn prop_mconcat_equivalent_to_fold(vec in proptest::collection::vec(any::<bool>(), 0..10)) {
      let mconcat_result = bool::mconcat(vec.clone());
      let fold_result = vec.into_iter().fold(bool::empty(), |acc, x| acc.combine(x));
      prop_assert_eq!(mconcat_result, fold_result);
    }

    #[test]
    fn prop_mconcat_equivalent_to_any(vec in proptest::collection::vec(any::<bool>(), 0..10)) {
      let mconcat_result = bool::mconcat(vec.clone());
      let any_result = vec.into_iter().any(|x| x);
      prop_assert_eq!(mconcat_result, any_result);
    }

    #[test]
    fn prop_associativity(a in any::<bool>(), b in any::<bool>(), c in any::<bool>()) {
      // (a ⊕ b) ⊕ c = a ⊕ (b ⊕ c)
      prop_assert_eq!(
        (a.combine(b)).combine(c),
        a.combine(b.combine(c))
      );
    }

    #[test]
    fn prop_combine_truth_table(a in any::<bool>(), b in any::<bool>()) {
      // For bool, combine should be logical OR
      prop_assert_eq!(a.combine(b), a || b);
    }
  }
}
