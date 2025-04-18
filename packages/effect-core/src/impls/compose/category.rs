use crate::{
  traits::category::Category,
  types::{compose::Compose, threadsafe::CloneableThreadSafe},
};

impl<A: CloneableThreadSafe, B: CloneableThreadSafe> Category<A, B> for Compose<A, B> {
  type Morphism<C: CloneableThreadSafe, D: CloneableThreadSafe> = Compose<C, D>;

  fn id<C: CloneableThreadSafe>() -> Self::Morphism<C, C> {
    Compose::new(|x| x, |x| x)
  }

  fn compose<C: CloneableThreadSafe, D: CloneableThreadSafe, E: CloneableThreadSafe>(
    f: Self::Morphism<C, D>,
    g: Self::Morphism<D, E>,
  ) -> Self::Morphism<C, E> {
    Compose::new(move |x| g.apply(f.apply(x)), |x| x)
  }

  fn arr<C: CloneableThreadSafe, D: CloneableThreadSafe, F>(f: F) -> Self::Morphism<C, D>
  where
    F: for<'a> Fn(&'a C) -> D + CloneableThreadSafe,
  {
    Compose::new(move |x| f(&x), |x| x)
  }

  fn first<C: CloneableThreadSafe, D: CloneableThreadSafe, E: CloneableThreadSafe>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(C, E), (D, E)> {
    Compose::new(move |(a, c)| (f.apply(a), c), |x| x)
  }

  fn second<C: CloneableThreadSafe, D: CloneableThreadSafe, E: CloneableThreadSafe>(
    f: Self::Morphism<C, D>,
  ) -> Self::Morphism<(E, C), (E, D)> {
    Compose::new(move |(c, a)| (c, f.apply(a)), |x| x)
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
    fn test_id_i32(x in any::<i32>()) {
      let id = <Compose<i32, i32> as Category<i32, i32>>::id::<i32>();
      assert_eq!(id.apply(x), x);
    }

    #[test]
    fn test_id_string(s in ".*") {
      let id = <Compose<String, String> as Category<String, String>>::id::<String>();
      assert_eq!(id.apply(s.to_string()), s);
    }

    #[test]
    fn test_compose_i32(
      x in any::<i32>(),
      f_idx in 0..I32_FUNCTIONS.len(),
      g_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let g = I32_FUNCTIONS[g_idx];

      let morphism_f = <Compose<i32, i32> as Category<i32, i32>>::arr(move |x: &i32| f(*x));
      let morphism_g = <Compose<i32, i32> as Category<i32, i32>>::arr(move |x: &i32| g(*x));

      let composed = <Compose<i32, i32> as Category<i32, i32>>::compose(morphism_f, morphism_g);
      assert_eq!(composed.apply(x), g(f(x)));
    }

    #[test]
    fn test_compose_string(
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len(),
      g_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let g = STRING_FUNCTIONS[g_idx];

      let morphism_f = <Compose<String, String> as Category<String, String>>::arr(move |x: &String| f(x.clone()));
      let morphism_g = <Compose<String, String> as Category<String, String>>::arr(move |x: &String| g(x.clone()));

      let composed = <Compose<String, String> as Category<String, String>>::compose(morphism_f, morphism_g);
      assert_eq!(composed.apply(s.to_string()), g(f(s.to_string())));
    }

    #[test]
    fn test_first_i32(
      x in any::<i32>(),
      s in ".*",
      f_idx in 0..I32_FUNCTIONS.len()
    ) {
      let f = I32_FUNCTIONS[f_idx];
      let morphism_f = <Compose<i32, i32> as Category<i32, i32>>::arr(move |x: &i32| f(*x));

      let first_f = <Compose<i32, i32> as Category<i32, i32>>::first::<i32, i32, String>(morphism_f);
      let input = (x, s.to_string());
      let expected = (f(x), s.to_string());

      assert_eq!(first_f.apply(input), expected);
    }

    #[test]
    fn test_second_string(
      x in any::<i32>(),
      s in ".*",
      f_idx in 0..STRING_FUNCTIONS.len()
    ) {
      let f = STRING_FUNCTIONS[f_idx];
      let morphism_f = <Compose<String, String> as Category<String, String>>::arr(move |x: &String| f(x.clone()));

      let second_f = <Compose<String, String> as Category<String, String>>::second::<String, String, i32>(morphism_f);
      let input = (x, s.to_string());
      let expected = (x, f(s.to_string()));

      assert_eq!(second_f.apply(input), expected);
    }
  }

  #[test]
  fn test_thread_safety() {
    use std::thread;

    let f = |x: i32| x.saturating_add(1);
    let morphism_f = <Compose<i32, i32> as Category<i32, i32>>::arr(move |x: &i32| f(*x));

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
