use crate::{
  traits::arrow::Arrow, types::morphism::Morphism, types::threadsafe::CloneableThreadSafe,
};

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Arrow<A, B> for Morphism<A, B> {
  fn arrow<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + CloneableThreadSafe,
  {
    Morphism::new(f)
  }

  fn split<
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
    E: CloneableThreadSafe,
    F: CloneableThreadSafe,
  >(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<C, D>,
  ) -> Self::Morphism<(A, C), (B, D)> {
    Morphism::new(move |(x, y): (A, C)| -> (B, D) { (f.apply(x), g.apply(y)) })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<A, C>,
  ) -> Self::Morphism<A, (B, C)> {
    Morphism::new(move |x: A| -> (B, C) {
      let x_clone = x.clone();
      (f.apply(x), g.apply(x_clone))
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::category::Category;
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
    fn test_arrow_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let morphism = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(f);
      assert_eq!(morphism.apply(x), f(x));
    }

    #[test]
    fn test_arrow_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let morphism = <Morphism<String, String> as Arrow<String, String>>::arrow(f);
      assert_eq!(morphism.apply(s.to_string()), f(s.to_string()));
    }

    #[test]
    fn test_split_i32(
      x in any::<i32>(),
      y in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];

      let morphism_f = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(f);
      let morphism_g = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(g);

      let split = <Morphism<i32, i32> as Arrow<i32, i32>>::split::<i32, i32, (), ()>(morphism_f, morphism_g);

      let result = split.apply((x, y));
      let expected = (f(x), g(y));

      assert_eq!(result, expected);
    }

    #[test]
    fn test_fanout_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];

      let morphism_f = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(f);
      let morphism_g = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(g);

      let fanout = <Morphism<i32, i32> as Arrow<i32, i32>>::fanout(morphism_f, morphism_g);

      let result = fanout.apply(x);
      let expected = (f(x), g(x));

      assert_eq!(result, expected);
    }

    #[test]
    fn test_arrow_composition(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];

      let morphism_f = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(f);
      let morphism_g = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(g);

      // Using Category::compose, which should work with Arrow too
      let composed = <Morphism<i32, i32> as Category<i32, i32>>::compose(morphism_f, morphism_g);

      let result = composed.apply(x);
      let expected = g(f(x));

      assert_eq!(result, expected);
    }
  }

  #[test]
  fn test_arrow_laws() {
    // Test basic Arrow law: first(arr(f)) = arr(first(f))
    // Where first(f) (a, c) = (f(a), c)

    let f = |x: i32| x.saturating_add(10);
    let x = 5;
    let c = "hello".to_string();

    // Create two different ways of applying f to the first component of a pair
    let arrow_f = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(f);
    let first_arrow_f = <Morphism<i32, i32> as Category<i32, i32>>::first(arrow_f);

    // Create a direct arrow that applies f to the first component
    let first_f = move |pair: (i32, String)| (f(pair.0), pair.1);
    let arrow_first_f = <Morphism<(i32, String), (i32, String)> as Arrow<
      (i32, String),
      (i32, String),
    >>::arrow(first_f);

    // Check if they produce the same result
    let pair = (x, c.clone());
    assert_eq!(first_arrow_f.apply(pair.clone()), arrow_first_f.apply(pair));
  }

  #[test]
  fn test_thread_safety() {
    use std::thread;

    let f = |x: i32| x.saturating_add(1);
    let morphism_f = <Morphism<i32, i32> as Arrow<i32, i32>>::arrow(f);

    let handles: Vec<_> = (0..10)
      .map(|_| {
        let morphism_f = morphism_f.clone();
        thread::spawn(move || {
          assert_eq!(morphism_f.apply(1), 2);
        })
      })
      .collect();

    for handle in handles {
      handle.join().unwrap();
    }
  }
}
