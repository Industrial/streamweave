//! Provides the [`Compose`] struct for composing two functions in a thread-safe, type-safe manner.
//!
//! [`Compose`] allows you to combine two functions, `f` and `g`, such that the result of `f` is passed to `g`.
//! Both functions are required to be thread-safe, and the composition is stored using `Arc` for safe sharing across threads.

use std::sync::Arc;

use crate::types::threadsafe::ThreadSafe;

/// A composable pair of functions, allowing the composition of two thread-safe functions.
///
/// This struct wraps two functions `f: A -> B` and `g: B -> C`, providing a way to compose them
/// into a single function `h: A -> C` where `h(x) = g(f(x))`.
///
/// # Type Parameters
/// - `A`: Input type for the first function, must implement [`ThreadSafe`].
/// - `B`: Output type for the first function and input type for the second function, must implement [`ThreadSafe`].
/// - `C`: Output type for the second function, must implement [`ThreadSafe`].
///
/// # Examples
/// ```
/// use effect_core::types::compose::Compose;
/// let add_one = |x: i32| x + 1;
/// let double = |x: i32| x * 2;
/// let compose = Compose::new(add_one, double);
/// assert_eq!(compose.apply(5), 12); // (5 + 1) * 2 = 12
/// ```
#[derive(Clone)]
pub struct Compose<A: ThreadSafe, B: ThreadSafe> {
  pub(crate) f: Arc<dyn Fn(A) -> B + Send + Sync>,
  pub(crate) g: Arc<dyn Fn(B) -> B + Send + Sync>,
}

impl<A: ThreadSafe, B: ThreadSafe> Compose<A, B> {
  /// Creates a new [`Compose`] from two thread-safe functions.
  ///
  /// # Arguments
  /// * `f` - The first function, mapping from `A` to `B`.
  /// * `g` - The second function, mapping from `B` to `B`.
  ///
  /// # Returns
  /// A new [`Compose`] instance that applies `f` and then `g`.
  pub fn new<F, G>(f: F, g: G) -> Self
  where
    F: Fn(A) -> B + ThreadSafe,
    G: Fn(B) -> B + ThreadSafe,
  {
    Self {
      f: Arc::new(f),
      g: Arc::new(g),
    }
  }

  /// Applies the composed functions to the input value.
  ///
  /// # Arguments
  /// * `x` - The input value of type `A`.
  ///
  /// # Returns
  /// The result of applying `f` to `x`, then `g` to the result.
  pub fn apply(&self, x: A) -> B {
    (self.g)((self.f)(x))
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
    |s| format!("{s}a"),           // Append 'a'
    |s| s.repeat(2),               // Duplicate
    |s| s.chars().rev().collect(), // Reverse
    |s| s.to_uppercase(),          // Uppercase
    |s| s.to_lowercase(),          // Lowercase
    |s| s.trim().to_string(),      // Trim
  ];

  proptest! {
    #[test]
    fn test_new_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];
      let compose = Compose::new(f, g);
      assert_eq!(compose.apply(x), g(f(x)));
    }

    #[test]
    fn test_new_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len(),
      g_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let g = STRING_FUNCTIONS[g_idx];
      let compose = Compose::new(f, g);
      assert_eq!(compose.apply(s.to_string()), g(f(s.to_string())));
    }
  }

  #[test]
  fn test_edge_cases_i32() {
    // Test with i32::MAX
    let compose = Compose::new(|x: i32| x.saturating_add(1), |x: i32| x.saturating_mul(2));
    assert_eq!(compose.apply(i32::MAX), i32::MAX);

    // Test with i32::MIN
    let compose = Compose::new(|x: i32| x.saturating_sub(1), |x: i32| x.saturating_mul(2));
    assert_eq!(compose.apply(i32::MIN), i32::MIN);

    // Test with zero
    let compose = Compose::new(
      |x: i32| x.checked_div(2).unwrap_or(0),
      |x: i32| x.checked_mul(x).unwrap_or(i32::MAX),
    );
    assert_eq!(compose.apply(0), 0);
  }

  #[test]
  fn test_edge_cases_string() {
    // Test with empty string
    let compose = Compose::new(|s: String| format!("{s}a"), |s: String| s.repeat(2));
    assert_eq!(compose.apply("".to_string()), "aa");

    // Test with whitespace
    let compose = Compose::new(
      |s: String| s.trim().to_string(),
      |s: String| s.to_uppercase(),
    );
    assert_eq!(compose.apply("   test   ".to_string()), "TEST");

    // Test with unicode
    let compose = Compose::new(
      |s: String| s.chars().rev().collect(),
      |s: String| s.to_uppercase(),
    );
    assert_eq!(compose.apply("café".to_string()), "ÉFAC");
  }

  #[test]
  fn test_thread_safety() {
    use std::thread;

    let compose = Compose::new(|x: i32| x.saturating_add(1), |x: i32| x.saturating_mul(2));

    let handles: Vec<_> = (0..10)
      .map(|_| {
        let compose = compose.clone();
        thread::spawn(move || {
          assert_eq!(compose.apply(1), 4); // (1 + 1) * 2
        })
      })
      .collect();

    for handle in handles {
      handle.join().unwrap();
    }
  }

  #[test]
  fn test_compose_identity() {
    let compose = Compose::new(|x: i32| x.saturating_add(1), |x: i32| x.saturating_mul(2));
    assert_eq!(compose.apply(1), 4); // (1 + 1) * 2 = 4
  }

  #[test]
  fn test_compose_string() {
    let compose = Compose::new(
      |s: String| s.trim().to_string(),
      |s: String| s.to_uppercase(),
    );
    assert_eq!(compose.apply("  hello  ".to_string()), "HELLO");
  }
}
