use crate::{traits::category::Category, types::threadsafe::CloneableThreadSafe};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// A cloneable function wrapper for Future
#[derive(Clone)]
pub struct FutureFn<A, B>(
  Arc<
    dyn Fn(Box<dyn Future<Output = A> + Send + Unpin>) -> Pin<Box<dyn Future<Output = B> + Send>>
      + Send
      + Sync
      + 'static,
  >,
);

impl<A, B> FutureFn<A, B> {
  pub fn new<F>(f: F) -> Self
  where
    F: Fn(Box<dyn Future<Output = A> + Send + Unpin>) -> Pin<Box<dyn Future<Output = B> + Send>>
      + Send
      + Sync
      + 'static,
  {
    FutureFn(Arc::new(f))
  }

  pub fn apply(
    &self,
    a: Box<dyn Future<Output = A> + Send + Unpin>,
  ) -> Pin<Box<dyn Future<Output = B> + Send>> {
    (self.0)(a)
  }
}

// We can't implement traits for Future directly, so we use a dummy type
#[derive(Clone)]
pub struct FutureCategory<T>(pub std::marker::PhantomData<T>);

impl<T: CloneableThreadSafe + Send + 'static> Category<T, T> for FutureCategory<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = FutureFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A> {
    FutureFn::new(|future| Box::pin(future))
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C> {
    FutureFn::new(move |future| {
      let f_clone = f.clone();
      let g_clone = g.clone();
      Box::pin(async move {
        let b = f_clone.apply(future).await;
        let b_fut = Box::new(std::future::ready(b));
        g_clone.apply(b_fut).await
      })
    })
  }

  fn arr<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: for<'a> Fn(&'a A) -> B + CloneableThreadSafe,
  {
    let f = Arc::new(f);
    FutureFn::new(move |a| {
      let f = Arc::clone(&f);
      Box::pin(async move {
        let a = a.await;
        f(&a)
      })
    })
  }

  fn first<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(A, C), (B, C)> {
    FutureFn::new(move |ac| {
      let f = f.clone();
      Box::pin(async move {
        let (a, c) = ac.await;
        let a_fut = Box::new(std::future::ready(a));
        let b = f.apply(a_fut).await;
        (b, c)
      })
    })
  }

  fn second<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
  ) -> Self::Morphism<(C, A), (C, B)> {
    FutureFn::new(move |ca| {
      let f = f.clone();
      Box::pin(async move {
        let (c, a) = ca.await;
        let a_fut = Box::new(std::future::ready(a));
        let b = f.apply(a_fut).await;
        (c, b)
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::traits::category::Category;

  // Define test functions that operate on integers - using i64 instead of i32
  // and using checked arithmetic to prevent overflow
  const INT_FUNCTIONS: &[fn(&i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if *x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(*x),
    |x| x.checked_neg().unwrap_or(i64::MAX),
  ];

  // Helper to convert a value to a boxed future
  fn boxed_future<T: Send + 'static>(value: T) -> Box<dyn Future<Output = T> + Send + Unpin> {
    Box::new(std::future::ready(value))
  }

  // Test the identity law: id() . f = f = f . id()
  #[tokio::test]
  async fn test_identity_law() {
    // Use a safe value that won't cause overflow
    let x = 42i64;

    for f_idx in 0..INT_FUNCTIONS.len() {
      let f = INT_FUNCTIONS[f_idx];

      // Create our Category::arr version of the function
      let arr_f = <FutureCategory<i64> as Category<i64, i64>>::arr(f);

      // Get the identity morphism
      let id = <FutureCategory<i64> as Category<i64, i64>>::id();

      // Compose id . f
      let id_then_f =
        <FutureCategory<i64> as Category<i64, i64>>::compose(id.clone(), arr_f.clone());

      // Compose f . id
      let f_then_id = <FutureCategory<i64> as Category<i64, i64>>::compose(arr_f.clone(), id);

      // Apply the input to each composition
      let result_f = arr_f.apply(boxed_future(x)).await;
      let result_id_then_f = id_then_f.apply(boxed_future(x)).await;
      let result_f_then_id = f_then_id.apply(boxed_future(x)).await;

      // All should give the same result
      assert_eq!(result_f, result_id_then_f);
      assert_eq!(result_f, result_f_then_id);
    }
  }

  // Test the composition law: (f . g) . h = f . (g . h)
  #[tokio::test]
  async fn test_composition_law() {
    // Use a safe value that won't cause overflow
    let x = 42i64;

    for f_idx in 0..INT_FUNCTIONS.len() {
      for g_idx in 0..INT_FUNCTIONS.len() {
        for h_idx in 0..INT_FUNCTIONS.len() {
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <FutureCategory<i64> as Category<i64, i64>>::arr(f);
          let arr_g = <FutureCategory<i64> as Category<i64, i64>>::arr(g);
          let arr_h = <FutureCategory<i64> as Category<i64, i64>>::arr(h);

          // Compose (f . g) . h
          let fg =
            <FutureCategory<i64> as Category<i64, i64>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <FutureCategory<i64> as Category<i64, i64>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <FutureCategory<i64> as Category<i64, i64>>::compose(arr_g, arr_h);
          let f_gh = <FutureCategory<i64> as Category<i64, i64>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(boxed_future(x)).await;
          let result_f_gh = f_gh.apply(boxed_future(x)).await;

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
        }
      }
    }
  }

  // Test the first combinator - specific to Future implementation (async)
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

  // Test the second combinator - specific to Future implementation (async)
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
}
