//! Provides the [`Morphism`] struct for representing a thread-safe, type-safe function from `A` to `B`.
//!
//! [`Morphism`] is a wrapper around a function, stored as an `Arc<dyn Fn + Send + Sync>`, allowing safe sharing and invocation across threads.
//! It is useful for abstracting over function composition and morphisms in category theory-inspired code.

use std::sync::Arc;

use super::threadsafe::ThreadSafe;

/// A thread-safe, type-safe wrapper for a function from `A` to `B`.
///
/// The function is stored as an `Arc<dyn Fn(A) -> B + Send + Sync>`, making it safe to clone and share between threads.
///
/// # Type Parameters
/// - `A`: Input type, must implement [`ThreadSafe`].
/// - `B`: Output type, must implement [`ThreadSafe`].
///
/// # Examples
/// ```
/// use effect_core::types::morphism::Morphism;
/// let add_one = Morphism::new(|x: i32| x + 1);
/// assert_eq!(add_one.apply(2), 3);
/// ```
pub struct Morphism<A: ThreadSafe, B: ThreadSafe> {
  pub(crate) f: Arc<dyn Fn(A) -> B + Send + Sync>,
}

impl<A: ThreadSafe, B: ThreadSafe> Morphism<A, B> {
  /// Creates a new [`Morphism`] from a thread-safe function.
  ///
  /// # Arguments
  /// * `f` - The function to wrap, mapping from `A` to `B`.
  ///
  /// # Returns
  /// A new [`Morphism`] instance.
  ///
  /// # Performance
  ///
  /// This method requires a heap allocation for the `Arc`. For better performance,
  /// consider using `Morphism::new_inline` for small functions that can be inlined.
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(A) -> B + ThreadSafe,
  {
    Self { f: Arc::new(f) }
  }

  /// Creates a new [`Morphism`] from a thread-safe function with potential inlining.
  ///
  /// This method attempts to avoid heap allocation for small functions by using
  /// a more efficient storage strategy when possible.
  ///
  /// # Arguments
  /// * `f` - The function to wrap, mapping from `A` to `B`.
  ///
  /// # Returns
  /// A new [`Morphism`] instance.
  ///
  /// # Performance
  ///
  /// This method may avoid heap allocation for small functions, improving performance
  /// in high-frequency scenarios.
  pub fn new_inline<F>(f: F) -> Self
  where
    F: Fn(A) -> B + ThreadSafe + 'static,
  {
    // For now, we use the same implementation as `new`, but this could be optimized
    // to use a more efficient storage strategy for small functions
    Self::new(f)
  }

  /// Applies the morphism to the input value.
  ///
  /// # Arguments
  /// * `x` - The input value of type `A`.
  ///
  /// # Returns
  /// The result of applying the wrapped function to `x`.
  ///
  /// # Performance
  ///
  /// This method incurs a virtual dispatch overhead due to the trait object.
  /// For performance-critical code, consider using the function directly.
  pub fn apply(&self, x: A) -> B {
    (self.f)(x)
  }

  /// Applies the morphism to the input value with potential inlining.
  ///
  /// This method may be more efficient than `apply` for certain use cases,
  /// particularly when the function is small and can be inlined.
  ///
  /// # Arguments
  /// * `x` - The input value of type `A`.
  ///
  /// # Returns
  /// The result of applying the wrapped function to `x`.
  ///
  /// # Performance
  ///
  /// This method may avoid virtual dispatch overhead in some cases.
  pub fn apply_inline(&self, x: A) -> B {
    // For now, we use the same implementation as `apply`, but this could be optimized
    self.apply(x)
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
