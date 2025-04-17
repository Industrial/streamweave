use std::sync::Arc;

pub struct Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> {
  pub(crate) f: Arc<dyn Fn(A) -> B + Send + Sync>,
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

  // Define test functions for i32 that are safe and easy to reason about
  const I32_FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x.saturating_add(1),                  // Safe increment
    |x| x.saturating_mul(2),                  // Safe doubling
    |x| x.saturating_sub(1),                  // Safe decrement
    |x| x.checked_div(2).unwrap_or(0),        // Safe halving
    |x| x.checked_mul(x).unwrap_or(i32::MAX), // Safe squaring
    |x| x.checked_neg().unwrap_or(i32::MIN),  // Safe negation
  ];

  // Define test functions for String that are safe and easy to reason about
  const STRING_FUNCTIONS: &[fn(String) -> String] = &[
    |s: String| format!("{s}a"),                     // Append 'a'
    |s: String| s.repeat(2),                         // Duplicate
    |s: String| s.chars().rev().collect::<String>(), // Reverse
    |s: String| s.to_uppercase(),                    // Uppercase
    |s: String| s.to_lowercase(),                    // Lowercase
    |s: String| s.trim().to_string(),                // Trim
  ];

  proptest! {
    #[test]
    fn test_new_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let morphism = Morphism::new(f);
      assert_eq!(morphism.apply(x), f(x));
    }

    #[test]
    fn test_new_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let morphism = Morphism::new(f);
      assert_eq!(morphism.apply(s.to_string()), f(s.to_string()));
    }
  }

  #[test]
  fn test_edge_cases_i32() {
    // Test with i32::MAX
    let morphism = Morphism::new(|x: i32| x.saturating_add(1));
    assert_eq!(morphism.apply(i32::MAX), i32::MAX);

    // Test with i32::MIN
    let morphism = Morphism::new(|x: i32| x.saturating_sub(1));
    assert_eq!(morphism.apply(i32::MIN), i32::MIN);

    // Test with zero
    let morphism = Morphism::new(|x: i32| x.checked_div(2).unwrap_or(0));
    assert_eq!(morphism.apply(0), 0);
  }

  #[test]
  fn test_edge_cases_string() {
    // Test with empty string
    let morphism = Morphism::new(|s: String| format!("{s}a"));
    assert_eq!(morphism.apply("".to_string()), "a");

    // Test with whitespace
    let morphism = Morphism::new(|s: String| s.trim().to_string());
    assert_eq!(morphism.apply("   test   ".to_string()), "test");

    // Test with unicode
    let morphism = Morphism::new(|s: String| s.chars().rev().collect::<String>());
    assert_eq!(morphism.apply("café".to_string()), "éfac");
  }

  #[test]
  fn test_thread_safety() {
    use std::thread;

    let morphism = Morphism::new(|x: i32| x.saturating_add(1));

    let handles: Vec<_> = (0..10)
      .map(|_| {
        let morphism = morphism.clone();
        thread::spawn(move || {
          assert_eq!(morphism.apply(1), 2);
        })
      })
      .collect();

    for handle in handles {
      handle.join().unwrap();
    }
  }
}
