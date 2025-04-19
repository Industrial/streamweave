use crate::traits::semigroup::Semigroup;

// Implement Semigroup for integer types
macro_rules! impl_integer_semigroup {
  ($($t:ty),*) => {
    $(
      impl Semigroup for $t {
        fn combine(self, other: Self) -> Self {
          self.wrapping_add(other)
        }
      }
    )*
  };
}

impl_integer_semigroup!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

// Implement Semigroup for floating-point types
macro_rules! impl_float_semigroup {
  ($($t:ty),*) => {
    $(
      impl Semigroup for $t {
        fn combine(self, other: Self) -> Self {
          self + other
        }
      }
    )*
  };
}

impl_float_semigroup!(f32, f64);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::proptest_semigroup_associativity;
  use crate::traits::semigroup::tests::test_associativity;
  use proptest::prelude::*;

  // Tests for integer types

  #[test]
  fn test_i32_combine() {
    assert_eq!(5i32.combine(10), 15);
    assert_eq!((-5i32).combine(10), 5);

    // Test wrapping behavior
    assert_eq!(i32::MAX.combine(1), i32::MIN);
    assert_eq!(i32::MIN.combine(-1), i32::MAX);
  }

  #[test]
  fn test_u32_combine() {
    assert_eq!(5u32.combine(10), 15);

    // Test wrapping behavior
    assert_eq!(u32::MAX.combine(1), 0);
  }

  #[test]
  fn test_i32_associativity() {
    test_associativity(1i32, 2i32, 3i32);
    test_associativity(-10i32, 5i32, 7i32);
  }

  #[test]
  fn test_u32_associativity() {
    test_associativity(1u32, 2u32, 3u32);
    test_associativity(10u32, 20u32, 30u32);
  }

  // Tests for floating-point types

  #[test]
  fn test_f64_combine() {
    assert_eq!(5.0f64.combine(10.0), 15.0);
    assert_eq!((-5.0f64).combine(10.0), 5.0);

    // Test special cases
    assert!(f64::INFINITY.combine(1.0).is_infinite());
    assert!(1.0f64.combine(f64::INFINITY).is_infinite());
    assert!(f64::NEG_INFINITY.combine(1.0).is_infinite());

    // NaN tests - using is_nan() since NaN != NaN
    assert!(f64::NAN.combine(1.0).is_nan());
    assert!(1.0f64.combine(f64::NAN).is_nan());
  }

  #[test]
  fn test_f64_associativity() {
    // Normal values
    test_associativity(1.0f64, 2.0f64, 3.0f64);
    test_associativity(-10.5f64, 5.25f64, 7.75f64);

    // With zeros
    test_associativity(0.0f64, 0.0f64, 0.0f64);
    test_associativity(-0.0f64, 0.0f64, 1.0f64);

    // Special cases with infinity require special handling
    // Not testing NaN since NaN != NaN
  }

  // Property-based tests

  // Integer associativity tests
  proptest_semigroup_associativity!(prop_i8_associativity, i8, any::<i8>());

  proptest_semigroup_associativity!(prop_i16_associativity, i16, any::<i16>());

  proptest_semigroup_associativity!(prop_i32_associativity, i32, any::<i32>());

  proptest_semigroup_associativity!(prop_i64_associativity, i64, -1_000_000..1_000_000i64);

  proptest_semigroup_associativity!(prop_u8_associativity, u8, any::<u8>());

  proptest_semigroup_associativity!(prop_u16_associativity, u16, any::<u16>());

  proptest_semigroup_associativity!(prop_u32_associativity, u32, any::<u32>());

  proptest_semigroup_associativity!(prop_u64_associativity, u64, 0..1_000_000u64);

  // Float associativity tests
  proptest! {
    #[test]
    fn prop_f32_associativity(
      // Filter out extremely large values (magnitude > 1e25) and non-normal values to avoid precision issues
      a in any::<f32>().prop_filter("Not normal or too large", |v| v.is_normal() && v.abs() < 1e25),
      b in any::<f32>().prop_filter("Not normal or too large", |v| v.is_normal() && v.abs() < 1e25),
      c in any::<f32>().prop_filter("Not normal or too large", |v| v.is_normal() && v.abs() < 1e25)
    ) {
      let left = a.combine(b).combine(c);
      let right = a.combine(b.combine(c));

      // Using a very large epsilon relative to the values being tested
      // The epsilon should be proportional to the magnitude of the operands
      let max_abs = a.abs().max(b.abs()).max(c.abs());
      let epsilon = max_abs * 1e-6;

      // Ensure we have a reasonable minimum epsilon even for very small values
      let epsilon = epsilon.max(1e-6);

      prop_assert!((left - right).abs() < epsilon,
                   "left={}, right={}, diff={}, epsilon={}",
                   left, right, (left - right).abs(), epsilon);
    }

    #[test]
    fn prop_f64_associativity(
      // Filter out extremely large values (magnitude > 1e200) and non-normal values to avoid precision issues
      a in any::<f64>().prop_filter("Not normal or too large", |v| v.is_normal() && v.abs() < 1e200),
      b in any::<f64>().prop_filter("Not normal or too large", |v| v.is_normal() && v.abs() < 1e200),
      c in any::<f64>().prop_filter("Not normal or too large", |v| v.is_normal() && v.abs() < 1e200)
    ) {
      // For floating-point, we need to account for epsilon differences
      let left = a.combine(b).combine(c);
      let right = a.combine(b.combine(c));

      // Using a relative epsilon based on the magnitude of the values
      let max_abs = a.abs().max(b.abs()).max(c.abs());
      let epsilon = max_abs * 1e-12;

      // Ensure we have a reasonable minimum epsilon even for very small values
      let epsilon = epsilon.max(1e-12);

      prop_assert!((left - right).abs() < epsilon,
                  "left={}, right={}, diff={}, epsilon={}",
                  left, right, (left - right).abs(), epsilon);
    }

    // Test for commutativity (a feature of addition, not required for semigroups in general)
    #[test]
    fn prop_numeric_commutativity(a in any::<i32>(), b in any::<i32>()) {
      prop_assert_eq!(a.combine(b), b.combine(a));
    }
  }
}
