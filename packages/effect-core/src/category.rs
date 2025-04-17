use crate::{compose::Compose, morphism::Morphism};

/// A category is a collection of objects and morphisms (arrows) between them.
///
/// # Safety
///
/// This trait requires that all implementations be thread-safe by default.
/// This means that:
/// - The type parameter T must implement Send + Sync + 'static
/// - The type parameter U must implement Send + Sync + 'static
/// - The composition function must implement Send + Sync + 'static
pub trait Category<T: Send + Sync + 'static, U: Send + Sync + 'static> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static>: Send + Sync + 'static;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A>;

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>;

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static;

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)>;

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)>;
}

impl<A: Send + Sync + 'static, B: Send + Sync + 'static> Category<A, B> for Morphism<A, B> {
  type Morphism<C: Send + Sync + 'static, D: Send + Sync + 'static> = Morphism<C, D>;

  fn id<C: Send + Sync + 'static>() -> Self::Morphism<C, C> {
    Morphism::new(|x| x)
  }

  fn compose<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
    g: Self::Morphism<D, E>,
  ) -> Self::Morphism<C, E> {
    Morphism::new(move |x| g.apply(f.apply(x)))
  }

  fn arr<C: Send + Sync + 'static, D: Send + Sync + 'static, F>(f: F) -> Self::Morphism<C, D>
  where
    F: Fn(C) -> D + Send + Sync + 'static,
  {
    Morphism::new(f)
  }

  fn first<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(C, E), (D, E)> {
    Morphism::new(move |(x, y)| (f.apply(x), y))
  }

  fn second<C: Send + Sync + 'static, D: Send + Sync + 'static, E: Send + Sync + 'static>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(E, C), (E, D)> {
    Morphism::new(move |(x, y)| (x, f.apply(y)))
  }
}

impl Category<(), ()> for Compose<(), ()> {
  type Morphism<A: Send + Sync + 'static, B: Send + Sync + 'static> = Compose<A, B>;

  fn id<A: Send + Sync + 'static>() -> Self::Morphism<A, A> {
    Compose::new(|x| x, |x| x)
  }

  fn compose<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    Compose::new(move |x| g.apply(f.apply(x)), |x| x)
  }

  fn arr<A: Send + Sync + 'static, B: Send + Sync + 'static, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + Send + Sync + 'static,
  {
    Compose::new(f, |x| x)
  }

  fn first<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    Compose::new(move |(a, c)| (f.apply(a), c), |x| x)
  }

  fn second<A: Send + Sync + 'static, B: Send + Sync + 'static, C: Send + Sync + 'static>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    Compose::new(move |(c, a)| (c, f.apply(a)), |x| x)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions for i32
  const I32_FUNCTIONS: &[fn(i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  // Define test functions for String
  const STRING_FUNCTIONS: &[fn(String) -> String] = &[
    |s| s + "a",
    |s| s.repeat(2),
    |s| s.chars().rev().collect(),
    |s| s.to_uppercase(),
    |s| s.to_lowercase(),
    |s| s.trim().to_string(),
  ];

  // Define test functions for tuples
  const TUPLE_FUNCTIONS: &[fn((i32, String)) -> (i32, String)] = &[
    |(x, s)| (x + 1, s + "a"),
    |(x, s)| (x * 2, s.repeat(2)),
    |(x, s)| (x - 1, s.chars().rev().collect()),
  ];

  proptest! {
      #[test]
      fn test_identity_laws_i32(x in any::<i32>()) {
          let id = <Morphism<i32, i32> as Category<i32, i32>>::id::<i32>();
          assert_eq!(id.apply(x), x);
      }

      #[test]
      fn test_identity_laws_string(s in ".*") {
          let id = <Morphism<String, String> as Category<String, String>>::id::<String>();
          assert_eq!(id.apply(s.to_string()), s);
      }

      #[test]
      fn test_identity_laws_tuple(
          x in any::<i32>(),
          s in ".*"
      ) {
          let id = <Morphism<(i32, String), (i32, String)> as Category<(i32, String), (i32, String)>>::id::<(i32, String)>();
          let input = (x, s.to_string());
          assert_eq!(id.apply(input.clone()), input);
      }

      #[test]
      fn test_composition_laws_i32(
          x in any::<i32>(),
          f_idx in 0..I32_FUNCTIONS.len(),
          g_idx in 0..I32_FUNCTIONS.len()
      ) {
          let f = I32_FUNCTIONS[f_idx];
          let g = I32_FUNCTIONS[g_idx];

          let morphism_f = <Morphism<i32, i32> as Category<i32, i32>>::arr(f);
          let morphism_g = <Morphism<i32, i32> as Category<i32, i32>>::arr(g);

          let composed = <Morphism<i32, i32> as Category<i32, i32>>::compose(morphism_f, morphism_g);
          assert_eq!(composed.apply(x), g(f(x)));
      }

      #[test]
      fn test_composition_laws_string(
          s in ".*",
          f_idx in 0..STRING_FUNCTIONS.len(),
          g_idx in 0..STRING_FUNCTIONS.len()
      ) {
          let f = STRING_FUNCTIONS[f_idx];
          let g = STRING_FUNCTIONS[g_idx];

          let morphism_f = <Morphism<String, String> as Category<String, String>>::arr(f);
          let morphism_g = <Morphism<String, String> as Category<String, String>>::arr(g);

          let composed = <Morphism<String, String> as Category<String, String>>::compose(morphism_f, morphism_g);
          assert_eq!(composed.apply(s.to_string()), g(f(s.to_string())));
      }

      #[test]
      fn test_first_laws(
          x in any::<i32>(),
          s in ".*",
          f_idx in 0..I32_FUNCTIONS.len()
      ) {
          let f = I32_FUNCTIONS[f_idx];
          let morphism_f = <Morphism<i32, i32> as Category<i32, i32>>::arr(f);

          let first_f = <Morphism<i32, i32> as Category<i32, i32>>::first::<i32, i32, String>(morphism_f);
          let input = (x, s.to_string());
          let expected = (f(x), s.to_string());

          assert_eq!(first_f.apply(input), expected);
      }

      #[test]
      fn test_second_laws(
          x in any::<i32>(),
          s in ".*",
          f_idx in 0..STRING_FUNCTIONS.len()
      ) {
          let f = STRING_FUNCTIONS[f_idx];
          let morphism_f = <Morphism<String, String> as Category<String, String>>::arr(f);

          let second_f = <Morphism<String, String> as Category<String, String>>::second::<String, String, i32>(morphism_f);
          let input = (x, s.to_string());
          let expected = (x, f(s.to_string()));

          assert_eq!(second_f.apply(input), expected);
      }

      #[test]
      fn test_arr_laws(
          x in any::<i32>(),
          f_idx in 0..I32_FUNCTIONS.len()
      ) {
          let f = I32_FUNCTIONS[f_idx];
          let morphism_f = <Morphism<i32, i32> as Category<i32, i32>>::arr(f);
          assert_eq!(morphism_f.apply(x), f(x));
      }

      #[test]
      fn test_compose_implementation(
          x in any::<i32>(),
          f_idx in 0..I32_FUNCTIONS.len(),
          g_idx in 0..I32_FUNCTIONS.len()
      ) {
          let f = I32_FUNCTIONS[f_idx];
          let g = I32_FUNCTIONS[g_idx];

          let morphism_f = <Compose<i32, i32> as Category<i32, i32>>::arr(f);
          let morphism_g = <Compose<i32, i32> as Category<i32, i32>>::arr(g);

          let composed = <Compose<i32, i32> as Category<i32, i32>>::compose(morphism_f, morphism_g);
          assert_eq!(composed.apply(x), g(f(x)));
      }
  }

  // Test thread safety explicitly
  #[test]
  fn test_thread_safety() {
    let f = |x: i32| x + 1;
    let morphism_f = <Morphism<i32, i32> as Category<i32, i32>>::arr(f);

    let handle = std::thread::spawn(move || morphism_f.apply(5));
    assert_eq!(handle.join().unwrap(), 6);
  }

  // Test edge cases explicitly
  #[test]
  fn test_edge_cases() {
    // Test with i32::MAX
    let f = |x: i32| x.checked_add(1).unwrap_or(i32::MAX);
    let g = |x: i32| x.checked_mul(2).unwrap_or(i32::MAX);

    let morphism_f = <Morphism<i32, i32> as Category<i32, i32>>::arr(f);
    let morphism_g = <Morphism<i32, i32> as Category<i32, i32>>::arr(g);

    let composed = <Morphism<i32, i32> as Category<i32, i32>>::compose(morphism_f, morphism_g);
    assert_eq!(composed.apply(i32::MAX - 1), i32::MAX);

    // Test with empty string
    let f = |s: String| s.len();
    let morphism_f = <Morphism<String, usize> as Category<String, usize>>::arr(f);
    assert_eq!(morphism_f.apply("".to_string()), 0);
  }
}
