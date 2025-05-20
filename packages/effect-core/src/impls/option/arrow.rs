use crate::impls::option::category::OptionFn;
use crate::{traits::arrow::Arrow, types::threadsafe::CloneableThreadSafe};

impl<T: CloneableThreadSafe> Arrow<T, T> for Option<T> {
  fn arrow<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + CloneableThreadSafe,
  {
    OptionFn::new(move |x: Option<A>| x.map(|a| f(a)))
  }

  fn split<
    A: CloneableThreadSafe,
    B: CloneableThreadSafe,
    C: CloneableThreadSafe,
    D: CloneableThreadSafe,
  >(
    f: Self::Morphism<T, T>,
    g: Self::Morphism<A, B>,
  ) -> Self::Morphism<(T, A), (T, B)> {
    OptionFn::new(move |pair: Option<(T, A)>| {
      pair.and_then(|(x, y)| {
        let opt_x = f.apply(Some(x));
        let opt_y = g.apply(Some(y));
        match (opt_x, opt_y) {
          (Some(new_x), Some(new_y)) => Some((new_x, new_y)),
          _ => None,
        }
      })
    })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<T, T>,
    g: Self::Morphism<T, C>,
  ) -> Self::Morphism<T, (T, C)> {
    OptionFn::new(move |x: Option<T>| {
      x.and_then(|value| {
        let opt_b = f.apply(Some(value.clone()));
        let opt_c = g.apply(Some(value));
        match (opt_b, opt_c) {
          (Some(b), Some(c)) => Some((b, c)),
          _ => None,
        }
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use proptest::prelude::*;

  // Define test functions that operate on integers
  const INT_FUNCTIONS: &[fn(i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Helper function to create an Option for testing
  fn make_option<T>(value: T, is_some: bool) -> Option<T> {
    if is_some {
      Some(value)
    } else {
      None
    }
  }

  proptest! {
    #[test]
    fn test_arrow_function(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      is_some in any::<bool>(),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let arrow_f = <Option<i64> as Arrow<i64, i64>>::arrow(f);

      let input = make_option(x, is_some);
      let result = arrow_f.apply(input);

      let expected = input.map(f);
      assert_eq!(result, expected);
    }

    #[test]
    fn test_split_functions(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      y in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      is_some in any::<bool>(),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <Option<i64> as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <Option<i64> as Arrow<i64, i64>>::arrow(g);

      let split = <Option<i64> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f, arrow_g);

      let input = make_option((x, y), is_some);
      let result = split.apply(input);

      let expected = input.map(|(a, b)| (f(a), g(b)));
      assert_eq!(result, expected);
    }

    #[test]
    fn test_fanout_functions(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      is_some in any::<bool>(),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <Option<i64> as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <Option<i64> as Arrow<i64, i64>>::arrow(g);

      let fanout = <Option<i64> as Arrow<i64, i64>>::fanout(arrow_f, arrow_g);

      let input = make_option(x, is_some);
      let result = fanout.apply(input);

      let expected = input.map(|a| (f(a), g(a)));
      assert_eq!(result, expected);
    }

    #[test]
    fn test_none_handling(
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <Option<i64> as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <Option<i64> as Arrow<i64, i64>>::arrow(g);

      // Test arrow with None
      assert_eq!(arrow_f.apply(None), None);

      // Test split with None
      let split = <Option<i64> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f.clone(), arrow_g.clone());
      assert_eq!(split.apply(None), None);

      // Test fanout with None
      let fanout = <Option<i64> as Arrow<i64, i64>>::fanout(arrow_f, arrow_g);
      assert_eq!(fanout.apply(None), None);
    }
  }

  #[test]
  fn test_arrow_law_split_preserves_arrows() {
    // Arrow law: split preserves arrows
    // split(arr(f), arr(g)) = arr(\(x,y) -> (f x, g y))

    let f = |x: i64| x + 10;
    let g = |y: i64| y * 2;

    let arrow_f = <Option<i64> as Arrow<i64, i64>>::arrow(f);
    let arrow_g = <Option<i64> as Arrow<i64, i64>>::arrow(g);

    let split_arrows =
      <Option<i64> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f, arrow_g);

    let combined_fn = move |pair: (i64, i64)| (f(pair.0), g(pair.1));
    let arrow_combined = <Option<(i64, i64)> as Arrow<(i64, i64), (i64, i64)>>::arrow(combined_fn);

    // Test with Some value
    let input = Some((5, 7));
    assert_eq!(
      split_arrows.apply(input.clone()),
      arrow_combined.apply(input)
    );

    // Test with None
    assert_eq!(split_arrows.apply(None), arrow_combined.apply(None));
  }
}
