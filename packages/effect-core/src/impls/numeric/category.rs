use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::sync::Arc;

/// A cloneable function wrapper for numeric types
#[derive(Clone)]
pub struct NumericFn<A, B>(Arc<dyn Fn(A) -> B + Send + Sync + 'static>);

impl<A, B> NumericFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    NumericFn(Arc::new(f))
  }

  pub fn apply(&self, a: A) -> B {
    (self.0)(a)
  }
}

// Implement Category for numeric types using a macro
macro_rules! impl_numeric_category {
  ($($t:ty),*) => {
    $(
      impl Category<$t, $t> for $t {
        type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = NumericFn<A, B>;

        fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
          NumericFn::new(|x| x)
        }

        fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
          f: Self::Morphism<A, B>,
          g: Self::Morphism<B, C>,
        ) -> Self::Morphism<A, C> {
          NumericFn::new(move |x| g.apply(f.apply(x)))
        }

        fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
        where
          F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
        {
          NumericFn::new(move |x| f(&x))
        }

        fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
          f: Self::Morphism<A, B>,
        ) -> Self::Morphism<(A, C), (B, C)> {
          NumericFn::new(move |(a, c)| (f.apply(a), c))
        }

        fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
          f: Self::Morphism<A, B>,
        ) -> Self::Morphism<(C, A), (C, B)> {
          NumericFn::new(move |(c, a)| (c, f.apply(a)))
        }
      }
    )*
  };
}

// Apply the macro to implement Category for integer and floating-point types
impl_numeric_category!(i32, i64, f32, f64);

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functions for numeric types with overflow protection
  fn double(x: &i32) -> i32 {
    x.saturating_mul(2)
  }

  fn increment(x: &i32) -> i32 {
    x.saturating_add(1)
  }

  fn square(x: &i32) -> i32 {
    // Prevent overflow by checking if squaring would overflow
    if *x > (i32::MAX as f64).sqrt() as i32 || *x < -((i32::MAX as f64).sqrt() as i32) {
      i32::MAX // Saturate at max value
    } else {
      x.saturating_mul(*x)
    }
  }

  fn double_i64(x: &i64) -> i64 {
    x.saturating_mul(2)
  }

  fn increment_i64(x: &i64) -> i64 {
    x.saturating_add(1)
  }

  fn square_i64(x: &i64) -> i64 {
    // Prevent overflow
    if *x > (i64::MAX as f64).sqrt() as i64 || *x < -((i64::MAX as f64).sqrt() as i64) {
      i64::MAX
    } else {
      x.saturating_mul(*x)
    }
  }

  fn double_f32(x: &f32) -> f32 {
    x * 2.0
  }

  fn increment_f32(x: &f32) -> f32 {
    x + 1.0
  }

  fn square_f32(x: &f32) -> f32 {
    x * x
  }

  fn half(x: &f64) -> f64 {
    x / 2.0
  }

  fn sqrt_abs(x: &f64) -> f64 {
    x.abs().sqrt()
  }

  // Individual test helpers for each type to avoid generic type bounds issues
  fn test_category_laws_i32(value: i32) {
    // Identity law
    let id = <i32 as Category<i32, i32>>::id::<i32>();
    assert_eq!(id.apply(value), value);

    // Left identity
    let f = <i32 as Category<i32, i32>>::arr(double);
    let id_clone = id.clone(); // Clone before passing
    let id_then_f = <i32 as Category<i32, i32>>::compose(id_clone, f.clone());
    assert_eq!(id_then_f.apply(value), f.apply(value));

    // Right identity
    let id_clone = id.clone(); // Clone before passing
    let f_then_id = <i32 as Category<i32, i32>>::compose(f.clone(), id_clone);
    assert_eq!(f_then_id.apply(value), f.apply(value));

    // Associativity
    let g = <i32 as Category<i32, i32>>::arr(square);

    // (f . g) . id should equal f . (g . id)
    let f_then_g = <i32 as Category<i32, i32>>::compose(f.clone(), g.clone());
    let id_clone = id.clone(); // Clone before passing
    let fg_then_id = <i32 as Category<i32, i32>>::compose(f_then_g, id_clone);

    let id_clone = id.clone(); // Clone before passing
    let g_then_id = <i32 as Category<i32, i32>>::compose(g, id_clone);
    let f_then_gid = <i32 as Category<i32, i32>>::compose(f, g_then_id);

    assert_eq!(fg_then_id.apply(value), f_then_gid.apply(value));
  }

  fn test_category_laws_i64(value: i64) {
    // Identity law
    let id = <i64 as Category<i64, i64>>::id::<i64>();
    assert_eq!(id.apply(value), value);

    // Left identity
    let f = <i64 as Category<i64, i64>>::arr(double_i64);
    let id_clone = id.clone(); // Clone before passing
    let id_then_f = <i64 as Category<i64, i64>>::compose(id_clone, f.clone());
    assert_eq!(id_then_f.apply(value), f.apply(value));

    // Right identity
    let id_clone = id.clone(); // Clone before passing
    let f_then_id = <i64 as Category<i64, i64>>::compose(f.clone(), id_clone);
    assert_eq!(f_then_id.apply(value), f.apply(value));

    // Associativity
    let g = <i64 as Category<i64, i64>>::arr(square_i64);

    // (f . g) . id should equal f . (g . id)
    let f_then_g = <i64 as Category<i64, i64>>::compose(f.clone(), g.clone());
    let id_clone = id.clone(); // Clone before passing
    let fg_then_id = <i64 as Category<i64, i64>>::compose(f_then_g, id_clone);

    let id_clone = id.clone(); // Clone before passing
    let g_then_id = <i64 as Category<i64, i64>>::compose(g, id_clone);
    let f_then_gid = <i64 as Category<i64, i64>>::compose(f, g_then_id);

    assert_eq!(fg_then_id.apply(value), f_then_gid.apply(value));
  }

  fn test_category_laws_f32(value: f32) {
    // Identity law
    let id = <f32 as Category<f32, f32>>::id::<f32>();
    assert_eq!(id.apply(value), value);

    // Left identity
    let f = <f32 as Category<f32, f32>>::arr(double_f32);
    let id_clone = id.clone(); // Clone before passing
    let id_then_f = <f32 as Category<f32, f32>>::compose(id_clone, f.clone());
    assert_eq!(id_then_f.apply(value), f.apply(value));

    // Right identity
    let id_clone = id.clone(); // Clone before passing
    let f_then_id = <f32 as Category<f32, f32>>::compose(f.clone(), id_clone);
    assert_eq!(f_then_id.apply(value), f.apply(value));

    // Associativity
    let g = <f32 as Category<f32, f32>>::arr(square_f32);

    // (f . g) . id should equal f . (g . id)
    let f_then_g = <f32 as Category<f32, f32>>::compose(f.clone(), g.clone());
    let id_clone = id.clone(); // Clone before passing
    let fg_then_id = <f32 as Category<f32, f32>>::compose(f_then_g, id_clone);

    let id_clone = id.clone(); // Clone before passing
    let g_then_id = <f32 as Category<f32, f32>>::compose(g, id_clone);
    let f_then_gid = <f32 as Category<f32, f32>>::compose(f, g_then_id);

    // For floating point, use a more lenient epsilon based on value magnitude
    let epsilon = value.abs() * 1e-5 + 1e-5;
    assert!(
      (fg_then_id.apply(value) - f_then_gid.apply(value)).abs() < epsilon,
      "Failed associativity test for value {}: {} vs {}",
      value,
      fg_then_id.apply(value),
      f_then_gid.apply(value)
    );
  }

  fn test_category_laws_f64(value: f64) {
    // Identity law
    let id = <f64 as Category<f64, f64>>::id::<f64>();
    assert_eq!(id.apply(value), value);

    // Left identity
    let f = <f64 as Category<f64, f64>>::arr(half);
    let id_clone = id.clone(); // Clone before passing
    let id_then_f = <f64 as Category<f64, f64>>::compose(id_clone, f.clone());
    assert_eq!(id_then_f.apply(value), f.apply(value));

    // Right identity
    let id_clone = id.clone(); // Clone before passing
    let f_then_id = <f64 as Category<f64, f64>>::compose(f.clone(), id_clone);
    assert_eq!(f_then_id.apply(value), f.apply(value));

    // Associativity
    let g = <f64 as Category<f64, f64>>::arr(sqrt_abs);

    // (f . g) . id should equal f . (g . id)
    let f_then_g = <f64 as Category<f64, f64>>::compose(f.clone(), g.clone());
    let id_clone = id.clone(); // Clone before passing
    let fg_then_id = <f64 as Category<f64, f64>>::compose(f_then_g, id_clone);

    let id_clone = id.clone(); // Clone before passing
    let g_then_id = <f64 as Category<f64, f64>>::compose(g, id_clone);
    let f_then_gid = <f64 as Category<f64, f64>>::compose(f, g_then_id);

    // For floating point, use approximate equality
    assert!((fg_then_id.apply(value) - f_then_gid.apply(value)).abs() < 1e-10);
  }

  #[test]
  fn test_i32_identity_law() {
    let n = 42;
    let id = <i32 as Category<i32, i32>>::arr(|&x| x);

    assert_eq!(id.apply(n), n);
  }

  #[test]
  fn test_i32_composition_law() {
    let n = 10;
    let f = <i32 as Category<i32, i32>>::arr(double);
    let g = <i32 as Category<i32, i32>>::arr(increment);

    let f_then_g = <i32 as Category<i32, i32>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(n));

    assert_eq!(f_then_g.apply(n), expected);
    assert_eq!(f_then_g.apply(10), 21); // double(10) = 20, then increment(20) = 21
  }

  #[test]
  fn test_i32_first() {
    let pair = (5, "test");
    let f = <i32 as Category<i32, i32>>::arr(square);
    let first_f = <i32 as Category<i32, i32>>::first(f);

    let result = first_f.apply(pair);
    assert_eq!(result, (25, "test"));
  }

  #[test]
  fn test_i32_second() {
    let pair = ("test", 5);
    let f = <i32 as Category<i32, i32>>::arr(square);
    let second_f = <i32 as Category<i32, i32>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result, ("test", 25));
  }

  #[test]
  fn test_i64_identity_law() {
    let n = 42i64;
    let id = <i64 as Category<i64, i64>>::arr(|&x| x);

    assert_eq!(id.apply(n), n);
  }

  #[test]
  fn test_i64_composition_law() {
    let n = 10i64;
    let f = <i64 as Category<i64, i64>>::arr(double_i64);
    let g = <i64 as Category<i64, i64>>::arr(increment_i64);

    let f_then_g = <i64 as Category<i64, i64>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(n));

    assert_eq!(f_then_g.apply(n), expected);
    assert_eq!(f_then_g.apply(10i64), 21i64);
  }

  #[test]
  fn test_i64_first() {
    let pair = (5i64, "test");
    let f = <i64 as Category<i64, i64>>::arr(square_i64);
    let first_f = <i64 as Category<i64, i64>>::first(f);

    let result = first_f.apply(pair);
    assert_eq!(result, (25i64, "test"));
  }

  #[test]
  fn test_i64_second() {
    let pair = ("test", 5i64);
    let f = <i64 as Category<i64, i64>>::arr(square_i64);
    let second_f = <i64 as Category<i64, i64>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result, ("test", 25i64));
  }

  #[test]
  fn test_f32_identity_law() {
    let n = std::f32::consts::PI;
    let id = <f32 as Category<f32, f32>>::arr(|&x| x);

    assert_eq!(id.apply(n), n);
  }

  #[test]
  fn test_f32_composition_law() {
    let n = 10.0f32;
    let f = <f32 as Category<f32, f32>>::arr(double_f32);
    let g = <f32 as Category<f32, f32>>::arr(increment_f32);

    let f_then_g = <f32 as Category<f32, f32>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(n));

    assert_eq!(f_then_g.apply(n), expected);
    assert_eq!(f_then_g.apply(10.0f32), 21.0f32);
  }

  #[test]
  fn test_f32_first() {
    let pair = (5.0f32, "test");
    let f = <f32 as Category<f32, f32>>::arr(square_f32);
    let first_f = <f32 as Category<f32, f32>>::first(f);

    let result = first_f.apply(pair);
    assert!((result.0 - 25.0f32).abs() < f32::EPSILON);
    assert_eq!(result.1, "test");
  }

  #[test]
  fn test_f32_second() {
    let pair = ("test", 5.0f32);
    let f = <f32 as Category<f32, f32>>::arr(square_f32);
    let second_f = <f32 as Category<f32, f32>>::second(f);

    let result = second_f.apply(pair);
    assert_eq!(result.0, "test");
    assert!((result.1 - 25.0f32).abs() < f32::EPSILON);
  }

  #[test]
  fn test_f64_identity_law() {
    let n = std::f64::consts::PI;
    let id = <f64 as Category<f64, f64>>::arr(|&x| x);

    assert_eq!(id.apply(n), n);
  }

  #[test]
  fn test_f64_composition_law() {
    let n = 16.0;
    let f = <f64 as Category<f64, f64>>::arr(half);
    let g = <f64 as Category<f64, f64>>::arr(sqrt_abs);

    let f_then_g = <f64 as Category<f64, f64>>::compose(f.clone(), g.clone());
    let expected = g.apply(f.apply(n));

    assert_eq!(f_then_g.apply(n), expected);

    // half(16) = 8, sqrt(8) = 2.82...
    let expected_sqrt = (8.0_f64).sqrt();
    assert!((f_then_g.apply(16.0) - expected_sqrt).abs() < 1e-10);
  }

  #[test]
  fn test_multiple_composition_i32() {
    // Test composing three functions
    let f1 = <i32 as Category<i32, i32>>::arr(double);
    let f2 = <i32 as Category<i32, i32>>::arr(increment);
    let f3 = <i32 as Category<i32, i32>>::arr(square);

    // ((double . increment) . square) vs (double . (increment . square))
    let f12 = <i32 as Category<i32, i32>>::compose(f1.clone(), f2.clone());
    let f123_left = <i32 as Category<i32, i32>>::compose(f12, f3.clone());

    let f23 = <i32 as Category<i32, i32>>::compose(f2, f3);
    let f123_right = <i32 as Category<i32, i32>>::compose(f1, f23);

    // Test associativity with multiple inputs
    for n in [0, 1, 5, 10, -5, i32::MAX / 2, i32::MIN / 2].iter() {
      assert_eq!(f123_left.apply(*n), f123_right.apply(*n));
    }
  }

  #[test]
  fn test_edge_cases_i32() {
    // Test with min/max values and zero
    let test_values = [0, 1, -1, i32::MAX / 2, i32::MIN / 2]; // Avoid extreme edges

    for &value in test_values.iter() {
      test_category_laws_i32(value);
    }
  }

  #[test]
  fn test_edge_cases_i64() {
    // Test with min/max values and zero
    let test_values = [0i64, 1i64, -1i64, i64::MAX / 2, i64::MIN / 2]; // Avoid extreme edges

    for &value in test_values.iter() {
      test_category_laws_i64(value);
    }
  }

  #[test]
  fn test_edge_cases_f32() {
    // Test with special float values
    let test_values = [
      0.0f32, 1.0f32, -1.0f32, 1000.0f32, -1000.0f32, // Use more reasonable values
    ];

    for &value in test_values.iter() {
      test_category_laws_f32(value);
    }

    // Test separately for infinity values
    let infinity_values = [f32::INFINITY, f32::NEG_INFINITY];
    for &value in infinity_values.iter() {
      // Skip full testing of category laws for infinity
      let id = <f32 as Category<f32, f32>>::id::<f32>();
      assert!(id.apply(value).is_infinite());
    }
  }

  #[test]
  fn test_edge_cases_f64() {
    // Test with special float values
    let test_values = [0.0f64, 1.0f64, -1.0f64, 1000.0f64, -1000.0f64];

    for &value in test_values.iter() {
      test_category_laws_f64(value);
    }

    // Special cases
    let special_values = [f64::EPSILON, -f64::EPSILON];
    for &value in special_values.iter() {
      // Just test identity law for very small values
      let id = <f64 as Category<f64, f64>>::id::<f64>();
      assert_eq!(id.apply(value), value);
    }
  }

  #[test]
  fn test_complex_composition_with_first_second() {
    // Test complex composition with first and second
    let f = <i32 as Category<i32, i32>>::arr(double);
    let g = <i32 as Category<i32, i32>>::arr(increment);

    // Create a function that applies double to the first element and increment to the second
    let f_first = <i32 as Category<i32, i32>>::first(f);
    let g_second = <i32 as Category<i32, i32>>::second(g);

    // Test with various inputs
    let input = (5, 10);
    let expected = (10, 11); // double(5) = 10, increment(10) = 11

    // First apply f to the first element, then apply g to the second element
    let result = g_second.apply(f_first.apply(input));

    assert_eq!(result, expected);
  }

  proptest! {
    #[test]
    fn prop_i32_identity_law(n in any::<i32>()) {
      let id = <i32 as Category<i32, i32>>::arr(|&x| x);
      prop_assert_eq!(id.apply(n), n);
    }

    #[test]
    fn prop_i32_composition_law(n in -100i32..100i32) { // Use a specific range instead of filtering
      let double_fn = <i32 as Category<i32, i32>>::arr(double);
      let inc_fn = <i32 as Category<i32, i32>>::arr(increment);

      let composed = <i32 as Category<i32, i32>>::compose(double_fn.clone(), inc_fn.clone());
      let composed_result = composed.apply(n);

      let expected = inc_fn.apply(double_fn.apply(n));
      prop_assert_eq!(composed_result, expected);
    }

    #[test]
    fn prop_i64_identity_law(n in any::<i64>()) {
      let id = <i64 as Category<i64, i64>>::arr(|&x| x);
      prop_assert_eq!(id.apply(n), n);
    }

    #[test]
    fn prop_i64_composition_law(n in -1000i64..1000i64) { // Use a specific range instead of filtering
      let double_fn = <i64 as Category<i64, i64>>::arr(double_i64);
      let inc_fn = <i64 as Category<i64, i64>>::arr(increment_i64);

      let composed = <i64 as Category<i64, i64>>::compose(double_fn.clone(), inc_fn.clone());
      let composed_result = composed.apply(n);

      let expected = inc_fn.apply(double_fn.apply(n));
      prop_assert_eq!(composed_result, expected);
    }

    #[test]
    fn prop_f32_identity_law(n in any::<f32>().prop_filter("Not NaN or Infinity", |n| n.is_finite())) {
      let id = <f32 as Category<f32, f32>>::arr(|&x| x);
      prop_assert_eq!(id.apply(n), n);
    }

    #[test]
    fn prop_f32_composition_law(n in -1000.0f32..1000.0f32) { // Use a specific range instead of filtering
      let double_fn = <f32 as Category<f32, f32>>::arr(double_f32);
      let inc_fn = <f32 as Category<f32, f32>>::arr(increment_f32);

      let composed = <f32 as Category<f32, f32>>::compose(double_fn.clone(), inc_fn.clone());
      let composed_result = composed.apply(n);

      let expected = inc_fn.apply(double_fn.apply(n));
      // Use approximate equality for floating point with more tolerance
      prop_assert!((composed_result - expected).abs() < 1e-3);
    }

    #[test]
    fn prop_f64_identity_law(n in any::<f64>().prop_filter("Not NaN or Infinity", |n| n.is_finite())) {
      let id = <f64 as Category<f64, f64>>::arr(|&x| x);
      prop_assert_eq!(id.apply(n), n);
    }

    #[test]
    fn prop_f64_composition_law(n in 0.0f64..1000.0f64) { // Use a specific range instead of filtering
      let half_fn = <f64 as Category<f64, f64>>::arr(half);
      let sqrt_fn = <f64 as Category<f64, f64>>::arr(sqrt_abs);

      let composed = <f64 as Category<f64, f64>>::compose(half_fn.clone(), sqrt_fn.clone());
      let composed_result = composed.apply(n);

      let expected = sqrt_fn.apply(half_fn.apply(n));
      prop_assert!((composed_result - expected).abs() < 1e-10);
    }

    #[test]
    fn prop_first_second_law_i32(a in -50i32..50i32, b in -50i32..50i32) { // Use a specific range instead of filtering
      let double_fn = <i32 as Category<i32, i32>>::arr(double);
      let inc_fn = <i32 as Category<i32, i32>>::arr(increment);

      // Test that first(f) then first(g) is equivalent to first(f.g)
      let first_double = <i32 as Category<i32, i32>>::first(double_fn.clone());
      let first_inc = <i32 as Category<i32, i32>>::first(inc_fn.clone());

      // Apply first to each function, then compose them
      let result1 = first_inc.apply(first_double.apply((a, b)));

      // Compose the functions, then apply first
      let double_then_inc = <i32 as Category<i32, i32>>::compose(double_fn, inc_fn);
      let first_composed = <i32 as Category<i32, i32>>::first(double_then_inc);
      let result2 = first_composed.apply((a, b));

      prop_assert_eq!(result1, result2);
    }

    #[test]
    fn prop_associativity_three_functions_i32(n in -20i32..20i32) { // Use a specific range instead of filtering
      let f1 = <i32 as Category<i32, i32>>::arr(double);
      let f2 = <i32 as Category<i32, i32>>::arr(increment);
      let f3 = <i32 as Category<i32, i32>>::arr(square);

      // (f1 . f2) . f3 should equal f1 . (f2 . f3)
      let f12 = <i32 as Category<i32, i32>>::compose(f1.clone(), f2.clone());
      let f123_left = <i32 as Category<i32, i32>>::compose(f12, f3.clone());

      let f23 = <i32 as Category<i32, i32>>::compose(f2, f3);
      let f123_right = <i32 as Category<i32, i32>>::compose(f1, f23);

      prop_assert_eq!(f123_left.apply(n), f123_right.apply(n));
    }
  }
}
