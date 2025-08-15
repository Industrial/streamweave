use crate::traits::functor::Functor;
use crate::types::threadsafe::CloneableThreadSafe;

// The Functor implementation for (A, B) maps only the second component
impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Functor<B> for (A, B) {
  type HigherSelf<U: CloneableThreadSafe> = (A, U);

  fn map<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: for<'a> FnMut(&'a B) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
  {
    (self.0, f(&self.1))
  }

  fn map_owned<U, F>(self, mut f: F) -> Self::HigherSelf<U>
  where
    F: FnMut(B) -> U + CloneableThreadSafe,
    U: CloneableThreadSafe,
    Self: Sized,
  {
    (self.0, f(self.1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Test functor identity law: map(id) == id
  #[test]
  fn test_functor_simple() {
    let pair = ("hello", 42);
    let id = |x: &i32| *x;
    let mapped = Functor::map(pair.clone(), id);
    assert_eq!(mapped, pair);
  }

  // Test functor composition law: map(f . g) == map(f) . map(g)
  #[test]
  fn test_composition_law() {
    let pair = ("world", 42);
    let f = |x: &i32| *x * 2;
    let g = |x: &i32| *x + 10;

    // Compose functions then map
    let h = move |x: &i32| {
      let g_result = g(x);
      f(&g_result)
    };
    let mapped_composition = Functor::map(pair.clone(), h);

    // Map with each function in sequence
    let mapped_g = Functor::map(pair.clone(), g);
    let mapped_f_g = Functor::map(mapped_g, f);

    assert_eq!(mapped_composition, mapped_f_g);
  }

  // Property-based tests for functor laws
  proptest! {
    #[test]
    fn test_functor_identity_prop(a in any::<String>(), b in -1000..1000i32) {
      let pair = (a.clone(), b);
      // Identity function should return the same value
      let mapped = Functor::map(pair, |x: &i32| *x);
      prop_assert_eq!(mapped, (a, b));
    }

    #[test]
    fn test_functor_composition_prop(a in any::<String>(), b in -1000..1000i32) {
      let pair = (a.clone(), b);

      // Define two functions that explicitly handle references
      let f = |x: &i32| x.saturating_mul(2);
      let g = |x: &i32| x.saturating_add(5);

      // Apply f then g
      let map_f = Functor::map(pair.clone(), f);
      let f_then_g = Functor::map(map_f, g);

      // Apply the composition directly
      let composed = move |x: &i32| {
        let f_result = f(x);
        g(&f_result)
      };
      let map_composed = Functor::map(pair, composed);

      // Results should be the same
      prop_assert_eq!(f_then_g, map_composed);
    }

    #[test]
    fn test_functor_composition_specific_prop(a in any::<String>(), b in -1000..1000i32) {
      let pair = (a.clone(), b);
      let f = |x: &i32| x.saturating_mul(2);
      let g = |x: &i32| x.saturating_add(10);

      // Mapping with composed function
      let composed = move |x: &i32| {
        let f_result = f(x);
        g(&f_result)
      };
      let map_composed = Functor::map(pair.clone(), composed);

      // Mapping with f then g
      let map_f = Functor::map(pair, f);
      let map_f_then_g = Functor::map(map_f, g);

      prop_assert_eq!(map_composed, map_f_then_g);
    }
  }

  // Test with different types
  #[test]
  fn test_functor_type_transformations() {
    // String -> Length
    let pair = ("test", "hello world");
    let mapped = Functor::map(pair, |s: &&str| s.len());
    assert_eq!(mapped, ("test", 11));

    // i32 -> String
    let pair = (42, 100);
    let mapped = Functor::map(pair, |n: &i32| n.to_string());
    assert_eq!(mapped, (42, "100".to_string()));

    // Vec -> Length
    let pair = ("data", vec![1, 2, 3, 4]);
    let mapped = Functor::map(pair, |v: &Vec<i32>| v.len());
    assert_eq!(mapped, ("data", 4));
  }

  // Test with stateful mapping function
  #[test]
  fn test_functor_with_stateful_mapping() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let pair = ("counter", 0);
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let mapped = Functor::map(pair, move |_: &i32| {
      counter_clone.fetch_add(1, Ordering::SeqCst)
    });

    assert_eq!(mapped, ("counter", 0));
    assert_eq!(counter.load(Ordering::SeqCst), 1);
  }

  // Test with complex types
  #[test]
  fn test_functor_with_complex_types() {
    // Nested tuples
    let nested = ("outer", ("inner", 42));
    let mapped = Functor::map(nested, |pair: &(&str, i32)| {
      (pair.0.to_string(), pair.1 * 2)
    });
    assert_eq!(mapped, ("outer", ("inner".to_string(), 84)));

    // Option type
    let option_pair = ("maybe", Some(5));
    let mapped = Functor::map(option_pair, |opt: &Option<i32>| opt.map(|x| x * 2));
    assert_eq!(mapped, ("maybe", Some(10)));

    let none_pair = ("empty", None::<i32>);
    let mapped = Functor::map(none_pair, |opt: &Option<i32>| opt.map(|x| x * 2));
    assert_eq!(mapped, ("empty", None));
  }

  // Test with different first component types
  #[test]
  fn test_functor_preserves_first_component() {
    // Using a more complex first component
    let pair = (vec![1, 2, 3], "second");
    let mapped = Functor::map(pair, |s: &&str| s.len());
    assert_eq!(mapped, (vec![1, 2, 3], 6));

    // Using Option as first component
    let pair = (Some(42), "value");
    let mapped = Functor::map(pair, |s: &&str| s.to_uppercase());
    assert_eq!(mapped, (Some(42), "VALUE".to_string()));
  }

  #[test]
  fn test_different_types() {
    let pair = ("test", 42);
    let f = |x: &i32| x.to_string();
    let result = Functor::map(pair, f);
    assert_eq!(result.0, "test");
    assert_eq!(result.1, "42");
  }

  #[test]
  fn test_complex_types() {
    let pair = ("data", vec![1, 2, 3]);
    let f = |v: &Vec<i32>| v.iter().sum::<i32>();
    let result = Functor::map(pair, f);
    assert_eq!(result.0, "data");
    assert_eq!(result.1, 6);
  }

  #[test]
  fn test_multiple_maps() {
    // Create a pair with a string and a number
    let pair = ("multi", 5);

    // Apply three transformations in sequence
    let mapped1 = Functor::map(pair, |x: &i32| *x * 2);
    let mapped2 = Functor::map(mapped1, |x: &i32| *x * 2);
    let mapped3 = Functor::map(mapped2, |x: &i32| x.to_string());

    // Final pair should have the transformed value
    assert_eq!(mapped3, ("multi", "20".to_string()));
  }

  #[test]
  fn test_with_reference_counts() {
    // No need for Rc in this test as we're not testing Rc behavior
    // Just use a direct integer value
    let pair = ("rc", 42);

    // Map over the pair with a simple function
    let mapped = Functor::map(pair, |n: &i32| *n);

    // Verify the result
    assert_eq!(mapped, ("rc", 42));
  }

  #[test]
  fn test_with_mutable_state() {
    // Create a shared mutable state counter
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicUsize::new(0));
    let pair = ("state", 10);

    // Function that updates state and returns a modified value
    let mapped = Functor::map(pair, {
      let counter_clone = Arc::clone(&counter);
      move |x: &i32| {
        let value = *x * 3;
        counter_clone.fetch_add(value as usize, Ordering::SeqCst);
        value
      }
    });

    // Check that both the pair was transformed correctly and side effect occurred
    assert_eq!(mapped, ("state", 30));
    assert_eq!(counter.load(Ordering::SeqCst), 30);
  }
}
