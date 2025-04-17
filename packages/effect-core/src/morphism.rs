use std::sync::Arc;

pub struct Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  f: Arc<dyn Fn(A) -> B + Send + Sync>,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Morphism<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Self { f: Arc::new(f) }
  }

  pub fn apply(&self, x: A) -> B {
    (self.f)(x)
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
    fn test_identity_i32(x in any::<i32>()) {
      let id = Morphism::new(|x: i32| x);
      assert_eq!(id.apply(x), x);
    }

    #[test]
    fn test_identity_string(s in ".*") {
      let id = Morphism::new(|s: String| s);
      assert_eq!(id.apply(s.to_string()), s);
    }

    #[test]
    fn test_composition_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let morphism = Morphism::new(f);
      assert_eq!(morphism.apply(x), f(x));
    }

    #[test]
    fn test_composition_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let morphism = Morphism::new(f);
      assert_eq!(morphism.apply(s.to_string()), f(s.to_string()));
    }
  }

  // Test edge cases explicitly
  #[test]
  fn test_edge_cases_i32() {
    // Test with i32::MAX
    let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
    let morphism = Morphism::new(f);
    assert_eq!(morphism.apply(i32::MAX - 1), i32::MAX);

    // Test with i32::MIN
    let f = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);
    let morphism = Morphism::new(f);
    assert_eq!(morphism.apply(i32::MIN + 1), i32::MIN);
  }

  #[test]
  fn test_edge_cases_string() {
    // Test with empty string
    let f = |s: String| s + "a";
    let morphism = Morphism::new(f);
    assert_eq!(morphism.apply("".to_string()), "a");

    // Test with string containing only whitespace
    let f = |s: String| s.trim().to_string();
    let morphism = Morphism::new(f);
    assert_eq!(morphism.apply("   ".to_string()), "");
  }

  // Test thread safety
  #[test]
  fn test_thread_safety() {
    let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
    let morphism = Morphism::new(f);

    let handle = std::thread::spawn(move || morphism.apply(5));
    assert_eq!(handle.join().unwrap(), 6);
  }
}
