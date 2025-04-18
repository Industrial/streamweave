use effect_core::impls::future::category::FutureCategory;
use effect_core::traits::category::Category;
use std::future::Future;

fn boxed_future<T: Send + 'static>(value: T) -> Box<dyn Future<Output = T> + Send + Unpin> {
  Box::new(std::future::ready(value))
}

// Define test functions that operate on integers
const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[|x| x.saturating_add(1), |x| x.saturating_mul(2)];

#[tokio::test]
async fn test_first_combinator() {
  // Use safe values that won't cause overflow
  let x = 42i64;
  let c = 10i64;

  for f_idx in 0..INT_FUNCTIONS.len() {
    let f = INT_FUNCTIONS[f_idx];

    // Create the arr version
    let arr_f = <FutureCategory<i64> as Category<i64, i64>>::arr(f);

    // Apply first to get a function on pairs
    let first_f = <FutureCategory<i64> as Category<i64, i64>>::first(arr_f);

    // Apply the first combinator
    let result = first_f.apply(boxed_future((x, c))).await;

    // The expected result is (f(x), c)
    let expected = (f(&x), c);

    // Results should match
    assert_eq!(result, expected);
  }
}

#[tokio::test]
async fn test_second_combinator() {
  // Use safe values that won't cause overflow
  let x = 42i64;
  let c = 10i64;

  for f_idx in 0..INT_FUNCTIONS.len() {
    let f = INT_FUNCTIONS[f_idx];

    // Create the arr version
    let arr_f = <FutureCategory<i64> as Category<i64, i64>>::arr(f);

    // Apply second to get a function on pairs
    let second_f = <FutureCategory<i64> as Category<i64, i64>>::second(arr_f);

    // Apply the second combinator
    let result = second_f.apply(boxed_future((c, x))).await;

    // The expected result is (c, f(x))
    let expected = (c, f(&x));

    // Results should match
    assert_eq!(result, expected);
  }
}
