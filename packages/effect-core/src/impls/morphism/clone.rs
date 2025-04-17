use crate::Morphism;

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Clone for Morphism<A, B> {
  fn clone(&self) -> Self {
    Self { f: self.f.clone() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions for i32
  const I32_FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x.checked_add(1).unwrap_or(i32::MAX), // Safe increment
    |x| x.checked_mul(2).unwrap_or(i32::MAX), // Safe doubling
    |x| x.checked_sub(1).unwrap_or(i32::MIN), // Safe decrement
    |x| x.checked_div(2).unwrap_or(i32::MIN), // Safe halving
    |x| x.checked_mul(x).unwrap_or(i32::MAX), // Safe squaring
    |x| x.checked_neg().unwrap_or(i32::MIN),  // Safe negation
  ];

  // Define test functions for String
  const STRING_FUNCTIONS: &[fn(String) -> String] = &[
    |s| s + "a",                   // Append 'a'
    |s| s.repeat(2),               // Double the string
    |s| s.chars().rev().collect(), // Reverse
    |s| s.to_uppercase(),          // Uppercase
    |s| s.to_lowercase(),          // Lowercase
    |s| s.trim().to_string(),      // Trim
  ];

  proptest! {
    #[test]
    fn test_clone_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];

      let original = Morphism::new(f);
      let cloned = original.clone();

      // Test that both produce the same output
      assert_eq!(original.apply(x), cloned.apply(x));

      // Test that they are independent
      let x2 = x.checked_add(1).unwrap_or(i32::MAX);
      assert_eq!(original.apply(x2), cloned.apply(x2));
    }

    #[test]
    fn test_clone_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];

      let original = Morphism::new(f);
      let cloned = original.clone();

      // Test that both produce the same output
      assert_eq!(original.apply(s.to_string()), cloned.apply(s.to_string()));

      // Test that they are independent
      let s2 = s.to_string() + "test";
      assert_eq!(original.apply(s2.clone()), cloned.apply(s2));
    }
  }

  #[test]
  fn test_clone_edge_cases() {
    // Test with i32::MAX
    let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
    let original = Morphism::new(f);
    let cloned = original.clone();
    assert_eq!(original.apply(i32::MAX - 1), cloned.apply(i32::MAX - 1));

    // Test with i32::MIN
    let f = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);
    let original = Morphism::new(f);
    let cloned = original.clone();
    assert_eq!(original.apply(i32::MIN + 1), cloned.apply(i32::MIN + 1));

    // Test with empty string
    let f = |s: String| s + "a";
    let original = Morphism::new(f);
    let cloned = original.clone();
    assert_eq!(original.apply("".to_string()), cloned.apply("".to_string()));

    // Test with whitespace string
    let f = |s: String| s.trim().to_string();
    let original = Morphism::new(f);
    let cloned = original.clone();
    assert_eq!(
      original.apply("   ".to_string()),
      cloned.apply("   ".to_string())
    );
  }
}
