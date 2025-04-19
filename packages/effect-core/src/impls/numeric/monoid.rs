use crate::traits::monoid::Monoid;

// Implement Monoid for integer types
macro_rules! impl_integer_monoid {
  ($($t:ty),*) => {
    $(
      impl Monoid for $t {
        fn empty() -> Self {
          0
        }
      }
    )*
  };
}

impl_integer_monoid!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

// Implement Monoid for floating-point types
macro_rules! impl_float_monoid {
  ($($t:ty),*) => {
    $(
      impl Monoid for $t {
        fn empty() -> Self {
          0.0
        }
      }
    )*
  };
}

impl_float_monoid!(f32, f64);

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::semigroup::Semigroup;
  use proptest::prelude::*;

  // Tests for integer types

  // Test empty() implementation for all integer types
  #[test]
  fn test_integer_empty() {
    assert_eq!(i8::empty(), 0i8);
    assert_eq!(i16::empty(), 0i16);
    assert_eq!(i32::empty(), 0i32);
    assert_eq!(i64::empty(), 0i64);
    assert_eq!(i128::empty(), 0i128);
    assert_eq!(isize::empty(), 0isize);

    assert_eq!(u8::empty(), 0u8);
    assert_eq!(u16::empty(), 0u16);
    assert_eq!(u32::empty(), 0u32);
    assert_eq!(u64::empty(), 0u64);
    assert_eq!(u128::empty(), 0u128);
    assert_eq!(usize::empty(), 0usize);
  }

  // Test left identity law for integer types
  #[test]
  fn test_integer_left_identity() {
    // Signed integers
    assert_eq!(i8::empty().combine(42i8), 42i8);
    assert_eq!(i8::empty().combine(-42i8), -42i8);

    assert_eq!(i16::empty().combine(42i16), 42i16);
    assert_eq!(i16::empty().combine(-42i16), -42i16);

    assert_eq!(i32::empty().combine(42i32), 42i32);
    assert_eq!(i32::empty().combine(-42i32), -42i32);

    assert_eq!(i64::empty().combine(42i64), 42i64);
    assert_eq!(i64::empty().combine(-42i64), -42i64);

    assert_eq!(i128::empty().combine(42i128), 42i128);
    assert_eq!(i128::empty().combine(-42i128), -42i128);

    assert_eq!(isize::empty().combine(42isize), 42isize);
    assert_eq!(isize::empty().combine(-42isize), -42isize);

    // Unsigned integers
    assert_eq!(u8::empty().combine(42u8), 42u8);
    assert_eq!(u16::empty().combine(42u16), 42u16);
    assert_eq!(u32::empty().combine(42u32), 42u32);
    assert_eq!(u64::empty().combine(42u64), 42u64);
    assert_eq!(u128::empty().combine(42u128), 42u128);
    assert_eq!(usize::empty().combine(42usize), 42usize);
  }

  // Test right identity law for integer types
  #[test]
  fn test_integer_right_identity() {
    // Signed integers
    assert_eq!(42i8.combine(i8::empty()), 42i8);
    assert_eq!((-42i8).combine(i8::empty()), -42i8);

    assert_eq!(42i16.combine(i16::empty()), 42i16);
    assert_eq!((-42i16).combine(i16::empty()), -42i16);

    assert_eq!(42i32.combine(i32::empty()), 42i32);
    assert_eq!((-42i32).combine(i32::empty()), -42i32);

    assert_eq!(42i64.combine(i64::empty()), 42i64);
    assert_eq!((-42i64).combine(i64::empty()), -42i64);

    assert_eq!(42i128.combine(i128::empty()), 42i128);
    assert_eq!((-42i128).combine(i128::empty()), -42i128);

    assert_eq!(42isize.combine(isize::empty()), 42isize);
    assert_eq!((-42isize).combine(isize::empty()), -42isize);

    // Unsigned integers
    assert_eq!(42u8.combine(u8::empty()), 42u8);
    assert_eq!(42u16.combine(u16::empty()), 42u16);
    assert_eq!(42u32.combine(u32::empty()), 42u32);
    assert_eq!(42u64.combine(u64::empty()), 42u64);
    assert_eq!(42u128.combine(u128::empty()), 42u128);
    assert_eq!(42usize.combine(usize::empty()), 42usize);
  }

  // Test identity with edge cases (min/max values)
  #[test]
  fn test_integer_identity_edge_cases() {
    // Test with min values
    assert_eq!(i8::MIN.combine(i8::empty()), i8::MIN);
    assert_eq!(i16::MIN.combine(i16::empty()), i16::MIN);
    assert_eq!(i32::MIN.combine(i32::empty()), i32::MIN);
    assert_eq!(i64::MIN.combine(i64::empty()), i64::MIN);
    assert_eq!(i128::MIN.combine(i128::empty()), i128::MIN);

    // Test with max values
    assert_eq!(i8::MAX.combine(i8::empty()), i8::MAX);
    assert_eq!(i16::MAX.combine(i16::empty()), i16::MAX);
    assert_eq!(i32::MAX.combine(i32::empty()), i32::MAX);
    assert_eq!(i64::MAX.combine(i64::empty()), i64::MAX);
    assert_eq!(i128::MAX.combine(i128::empty()), i128::MAX);

    assert_eq!(u8::MAX.combine(u8::empty()), u8::MAX);
    assert_eq!(u16::MAX.combine(u16::empty()), u16::MAX);
    assert_eq!(u32::MAX.combine(u32::empty()), u32::MAX);
    assert_eq!(u64::MAX.combine(u64::empty()), u64::MAX);
    assert_eq!(u128::MAX.combine(u128::empty()), u128::MAX);
  }

  // Test mconcat functionality with various permutations
  #[test]
  fn test_integer_mconcat() {
    // Empty vector
    let empty_vec: Vec<i32> = vec![];
    assert_eq!(i32::mconcat(empty_vec), 0);

    // Single element
    assert_eq!(i32::mconcat(vec![42]), 42);

    // Multiple elements
    assert_eq!(i32::mconcat(vec![1, 2, 3, 4, 5]), 15);

    // With negative numbers
    assert_eq!(i32::mconcat(vec![1, -2, 3, -4, 5]), 3);

    // Testing with wrapping behavior
    assert_eq!(i8::mconcat(vec![100, 100]), -56i8); // wraps around

    // Different orderings should yield same result with + operation
    assert_eq!(i32::mconcat(vec![1, 2, 3, 4]), 10);
    assert_eq!(i32::mconcat(vec![4, 3, 2, 1]), 10);
    assert_eq!(i32::mconcat(vec![1, 3, 2, 4]), 10);
  }

  // Tests for floating-point types

  // Test empty() implementation for floating-point types
  #[test]
  fn test_float_empty() {
    assert_eq!(f32::empty(), 0.0f32);
    assert_eq!(f64::empty(), 0.0f64);

    // Check sign (should be +0.0, not -0.0)
    assert!(!f32::empty().is_sign_negative());
    assert!(!f64::empty().is_sign_negative());
  }

  // Test left identity law for floating-point types
  #[test]
  fn test_float_left_identity() {
    // Regular values
    assert_eq!(f32::empty().combine(42.0f32), 42.0f32);
    assert_eq!(f32::empty().combine(-42.0f32), -42.0f32);

    assert_eq!(f64::empty().combine(42.0f64), 42.0f64);
    assert_eq!(f64::empty().combine(-42.0f64), -42.0f64);

    // Special values
    assert!(f32::empty().combine(f32::INFINITY).is_infinite());
    assert!(f32::empty().combine(f32::NEG_INFINITY).is_infinite());
    assert!(f32::empty().combine(f32::NAN).is_nan());

    assert!(f64::empty().combine(f64::INFINITY).is_infinite());
    assert!(f64::empty().combine(f64::NEG_INFINITY).is_infinite());
    assert!(f64::empty().combine(f64::NAN).is_nan());
  }

  // Test right identity law for floating-point types
  #[test]
  fn test_float_right_identity() {
    // Regular values
    assert_eq!(42.0f32.combine(f32::empty()), 42.0f32);
    assert_eq!((-42.0f32).combine(f32::empty()), -42.0f32);

    assert_eq!(42.0f64.combine(f64::empty()), 42.0f64);
    assert_eq!((-42.0f64).combine(f64::empty()), -42.0f64);

    // Special values
    assert!(f32::INFINITY.combine(f32::empty()).is_infinite());
    assert!(f32::NEG_INFINITY.combine(f32::empty()).is_infinite());
    assert!(f32::NAN.combine(f32::empty()).is_nan());

    assert!(f64::INFINITY.combine(f64::empty()).is_infinite());
    assert!(f64::NEG_INFINITY.combine(f64::empty()).is_infinite());
    assert!(f64::NAN.combine(f64::empty()).is_nan());
  }

  // Test mconcat functionality for floating-point types
  #[test]
  fn test_float_mconcat() {
    // Empty vector
    let empty_vec: Vec<f32> = vec![];
    assert_eq!(f32::mconcat(empty_vec), 0.0f32);

    // Single element
    assert_eq!(f32::mconcat(vec![42.5f32]), 42.5f32);

    // Multiple elements
    assert_eq!(f32::mconcat(vec![1.5, 2.0, 3.5, 4.0]), 11.0f32);

    // With negative numbers
    assert_eq!(f32::mconcat(vec![1.0, -2.0, 3.0, -4.0, 5.0]), 3.0f32);

    // Different orderings should yield approximately the same result
    // (floating-point addition isn't always exactly associative)
    let sum1 = f32::mconcat(vec![0.1, 0.2, 0.3, 0.4]);
    let sum2 = f32::mconcat(vec![0.4, 0.3, 0.2, 0.1]);
    assert!((sum1 - sum2).abs() < 1e-6);
  }

  // Property-based tests

  // Test that empty is a left identity
  proptest! {
    #[test]
    fn prop_i32_left_identity(x in any::<i32>()) {
      prop_assert_eq!(i32::empty().combine(x), x);
    }

    #[test]
    fn prop_i64_left_identity(x in any::<i64>()) {
      prop_assert_eq!(i64::empty().combine(x), x);
    }

    #[test]
    fn prop_f64_left_identity(x in any::<f64>()) {
      if x.is_nan() {
        // Special case for NaN since NaN != NaN
        prop_assert!(f64::empty().combine(x).is_nan());
      } else {
        prop_assert_eq!(f64::empty().combine(x), x);
      }
    }
  }

  // Test that empty is a right identity
  proptest! {
    #[test]
    fn prop_i32_right_identity(x in any::<i32>()) {
      prop_assert_eq!(x.combine(i32::empty()), x);
    }

    #[test]
    fn prop_i64_right_identity(x in any::<i64>()) {
      prop_assert_eq!(x.combine(i64::empty()), x);
    }

    #[test]
    fn prop_f64_right_identity(x in any::<f64>()) {
      if x.is_nan() {
        // Special case for NaN since NaN != NaN
        prop_assert!(x.combine(f64::empty()).is_nan());
      } else {
        prop_assert_eq!(x.combine(f64::empty()), x);
      }
    }
  }

  // Test that mconcat is equivalent to folding with combine
  proptest! {
    #[test]
    fn prop_i32_mconcat_equivalent_to_fold(xs in prop::collection::vec(any::<i32>(), 0..10)) {
      let mconcat_result = i32::mconcat(xs.clone());
      let fold_result = xs.into_iter().fold(i32::empty(), |acc, x| acc.combine(x));
      prop_assert_eq!(mconcat_result, fold_result);
    }

    #[test]
    fn prop_f64_mconcat_equivalent_to_fold(
      xs in prop::collection::vec(any::<f64>().prop_filter("Not NaN", |v| !v.is_nan()), 0..10)
    ) {
      let mconcat_result = f64::mconcat(xs.clone());
      let fold_result = xs.into_iter().fold(f64::empty(), |acc, x| acc.combine(x));

      // For floats we need to handle possible precision differences
      if mconcat_result.is_finite() && fold_result.is_finite() {
        prop_assert!((mconcat_result - fold_result).abs() < 1e-10);
      } else {
        // For special values, they should match exactly
        prop_assert_eq!(mconcat_result.is_infinite(), fold_result.is_infinite());
        prop_assert_eq!(mconcat_result.is_nan(), fold_result.is_nan());
      }
    }
  }
}
