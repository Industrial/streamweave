use std::sync::Arc;

pub struct Compose<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  f: Arc<dyn Fn(A) -> B + Send + Sync>,
  g: Arc<dyn Fn(B) -> B + Send + Sync>,
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Compose<A, B> {
  pub fn new<F, G>(f: F, g: G) -> Self
  where
    F: Fn(A) -> B + Send + Sync + 'static,
    G: Fn(B) -> B + Send + Sync + 'static,
  {
    Self {
      f: Arc::new(f),
      g: Arc::new(g),
    }
  }

  pub fn apply(&self, x: A) -> B {
    (self.g)((self.f)(x))
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
      let id = Compose::new(|x: i32| x, |x: i32| x);
      assert_eq!(id.apply(x), x);
    }

    #[test]
    fn test_identity_string(s in ".*") {
      let id = Compose::new(|s: String| s.clone(), |s: String| s);
      assert_eq!(id.apply(s.to_string()), s);
    }

    #[test]
    fn test_composition_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];

      let composed = Compose::new(f, g);
      assert_eq!(composed.apply(x), g(f(x)));
    }

    #[test]
    fn test_composition_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len(),
      g_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let g = STRING_FUNCTIONS[g_idx];

      let composed = Compose::new(f, g);
      assert_eq!(composed.apply(s.to_string()), g(f(s.to_string())));
    }

    #[test]
    fn test_edge_cases_i32(x in prop::collection::vec(prop::num::i32::ANY, 1)) {
      // Always test MAX case
      let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let g = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let composed = Compose::new(f, g);
      assert_eq!(composed.apply(i32::MAX - 1), i32::MAX);

      // Always test MIN case
      let f = |x: i32| x.checked_sub(1).unwrap_or(i32::MIN);
      let g = |x: i32| x.checked_div(2).unwrap_or(i32::MIN);
      let composed = Compose::new(f, g);
      // When we divide i32::MIN by 2, we get -1073741824
      assert_eq!(composed.apply(i32::MIN + 1), -1073741824);

      // Test with random value
      let f = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let g = |x: i32| x.checked_div(2).unwrap_or(i32::MIN);
      let composed = Compose::new(f, g);
      let test_val = x[0];
      // For values within safe range, composition should preserve value
      if test_val > i32::MIN / 2 && test_val < i32::MAX / 2 {
        assert_eq!(composed.apply(test_val), test_val);
      }
    }

    #[test]
    fn test_edge_cases_string(s in prop::string::string_regex(".{0,10}").unwrap()) {
      // Always test empty string
      let f = |s: String| s + "a";
      let g = |s: String| s.repeat(2);
      let composed = Compose::new(f, g);
      assert_eq!(composed.apply("".to_string()), "aa");

      // Always test whitespace string
      let f = |s: String| s.trim().to_string();
      let g = |s: String| s.to_uppercase();
      let composed = Compose::new(f, g);
      assert_eq!(composed.apply("   ".to_string()), "");

      // Test with random string
      let f = |s: String| s + "test";
      let g = |s: String| s.repeat(2);
      let composed = Compose::new(f, g);
      assert_eq!(composed.apply(s.clone()), (s + "test").repeat(2));
    }

    #[test]
    fn test_thread_safety(x in 1..100i32) {
      let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
      let g = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);
      let composed = Compose::new(f, g);

      let handle = std::thread::spawn(move || composed.apply(x));
      // For values that won't overflow, we can predict the result
      if x < (i32::MAX - 1) / 2 {
        assert_eq!(handle.join().unwrap(), (x + 1) * 2);
      }
    }
  }
}
