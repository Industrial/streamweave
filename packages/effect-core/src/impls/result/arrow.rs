use crate::{traits::arrow::Arrow, types::threadsafe::CloneableThreadSafe};
use crate::impls::result::category::ResultFn;

impl<T: CloneableThreadSafe, E: CloneableThreadSafe> Arrow<T, T> for Result<T, E> {
  fn arrow<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + CloneableThreadSafe,
  {
    ResultFn::new(move |x: Result<A, E>| x.map(|a| f(a)))
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
    ResultFn::new(move |pair: Result<(T, A), E>| {
      pair.and_then(|(x, y)| {
        match (f.apply(Ok(x)), g.apply(Ok(y))) {
          (Ok(new_x), Ok(new_y)) => Ok((new_x, new_y)),
          (Err(e), _) => Err(e),
          (_, Err(e)) => Err(e),
        }
      })
    })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<T, T>,
    g: Self::Morphism<T, C>,
  ) -> Self::Morphism<T, (T, C)> {
    ResultFn::new(move |x: Result<T, E>| {
      x.and_then(|value| {
        match (f.apply(Ok(value.clone())), g.apply(Ok(value))) {
          (Ok(b), Ok(c)) => Ok((b, c)),
          (Err(e), _) => Err(e),
          (_, Err(e)) => Err(e),
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

  // Helper function to create a Result for testing
  fn make_result<T>(value: T, err: String, is_ok: bool) -> Result<T, String> {
    if is_ok {
      Ok(value)
    } else {
      Err(err)
    }
  }

  proptest! {
    #[test]
    fn test_arrow_function(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      err in any::<String>(),
      is_ok in any::<bool>(),
      f_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let arrow_f = <Result<i64, String> as Arrow<i64, i64>>::arrow(f);

      let input = make_result(x, err.clone(), is_ok);
      let result = arrow_f.apply(input.clone());

      let expected = input.map(f);
      assert_eq!(result, expected);
    }

    #[test]
    fn test_split_functions(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      y in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      err in any::<String>(),
      is_ok in any::<bool>(),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <Result<i64, String> as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <Result<i64, String> as Arrow<i64, i64>>::arrow(g);

      let split = <Result<i64, String> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f, arrow_g);

      let input = make_result((x, y), err.clone(), is_ok);
      let result = split.apply(input.clone());

      // Expected behavior depends on whether input is Ok or Err
      let expected = if is_ok {
        Ok((f(x), g(y)))
      } else {
        Err(err)
      };

      assert_eq!(result, expected);
    }

    #[test]
    fn test_fanout_functions(
      x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      err in any::<String>(),
      is_ok in any::<bool>(),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <Result<i64, String> as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <Result<i64, String> as Arrow<i64, i64>>::arrow(g);

      let fanout = <Result<i64, String> as Arrow<i64, i64>>::fanout(arrow_f, arrow_g);

      let input = make_result(x, err.clone(), is_ok);
      let result = fanout.apply(input.clone());

      // Expected behavior depends on whether input is Ok or Err
      let expected = if is_ok {
        Ok((f(x), g(x)))
      } else {
        Err(err)
      };

      assert_eq!(result, expected);
    }

    #[test]
    fn test_error_handling(
      _x in any::<i64>().prop_filter("Value too large", |v| *v < 10000),
      err in any::<String>(),
      f_idx in 0..INT_FUNCTIONS.len(),
      g_idx in 0..INT_FUNCTIONS.len()
    ) {
      let f = INT_FUNCTIONS[f_idx];
      let g = INT_FUNCTIONS[g_idx];

      let arrow_f = <Result<i64, String> as Arrow<i64, i64>>::arrow(f);
      let arrow_g = <Result<i64, String> as Arrow<i64, i64>>::arrow(g);

      // Test arrow with Err
      let err_input = Err(err.clone());
      assert_eq!(arrow_f.apply(err_input.clone()), err_input);

      // Test split with Err
      let split = <Result<i64, String> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f.clone(), arrow_g.clone());
      let err_pair = Err(err.clone());
      assert_eq!(split.apply(err_pair.clone()), err_pair);

      // Test fanout with Err
      let fanout = <Result<i64, String> as Arrow<i64, i64>>::fanout(arrow_f, arrow_g);
      assert_eq!(fanout.apply(Err(err.clone())), Err(err));
    }
  }

  #[test]
  fn test_arrow_law_split_preserves_arrows() {
    // Arrow law: split preserves arrows
    // split(arr(f), arr(g)) = arr(\(x,y) -> (f x, g y))

    let f = |x: i64| x + 10;
    let g = |y: i64| y * 2;

    let arrow_f = <Result<i64, String> as Arrow<i64, i64>>::arrow(f);
    let arrow_g = <Result<i64, String> as Arrow<i64, i64>>::arrow(g);

    let split_arrows = <Result<i64, String> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f, arrow_g);

    let combined_fn = move |pair: (i64, i64)| (f(pair.0), g(pair.1));
    let arrow_combined = <Result<(i64, i64), String> as Arrow<(i64, i64), (i64, i64)>>::arrow(combined_fn);

    // Test with Ok value
    let input_ok = Ok((5, 7));
    assert_eq!(split_arrows.apply(input_ok.clone()), arrow_combined.apply(input_ok));

    // Test with Err value
    let input_err = Err("test error".to_string());
    assert_eq!(split_arrows.apply(input_err.clone()), arrow_combined.apply(input_err));
  }
} 