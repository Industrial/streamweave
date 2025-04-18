use crate::{threadsafe::CloneableThreadSafe, traits::Category};
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
pub struct FutureCategory<T>(std::marker::PhantomData<T>);

impl<T: CloneableThreadSafe + Send + 'static> Category<T, T> for FutureCategory<T> {
  type Morphism<A: CloneableThreadSafe, B: CloneableThreadSafe> = FutureFn<A, B>;

  fn id<A: CloneableThreadSafe>() -> Self::Morphism<A, A>
  where
    A: Send + 'static,
  {
    FutureFn::new(|future| Box::pin(future))
  }

  fn compose<A: CloneableThreadSafe, B: CloneableThreadSafe, C: CloneableThreadSafe>(
    f: Self::Morphism<A, B>,
    g: Self::Morphism<B, C>,
  ) -> Self::Morphism<A, C>
  where
    A: Send + 'static,
    B: Send + 'static,
    C: Send + 'static,
  {
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
    A: Send + 'static,
    B: Send + 'static,
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
  ) -> Self::Morphism<(A, C), (B, C)>
  where
    A: Send + 'static,
    B: Send + 'static,
    C: Send + 'static,
  {
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
  ) -> Self::Morphism<(C, A), (C, B)>
  where
    A: Send + 'static,
    B: Send + 'static,
    C: Send + 'static,
  {
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
  use crate::traits::Category;
  use std::future::ready;

  // Define test functions that operate on integers
  const INT_FUNCTIONS: &[fn(&i32) -> i32] = &[
    |x| x + 1,
    |x| x * 2,
    |x| x - 1,
    |x| x / 2,
    |x| x * x,
    |x| -x,
  ];

  // Helper to create a boxed ready future
  fn boxed_future<T: Send + 'static>(val: T) -> Box<dyn Future<Output = T> + Send + Unpin> {
    Box::new(ready(val))
  }

  // Helper to run a future and get its output
  async fn run_future<T>(future: Pin<Box<dyn Future<Output = T> + Send>>) -> T {
    future.await
  }

  // Test the identity law: id() . f = f = f . id()
  #[tokio::test]
  async fn test_identity_law() {
    let x = 5;

    for f_idx in 0..INT_FUNCTIONS.len() {
      let f = INT_FUNCTIONS[f_idx];

      // Create our Category::arr version of the function
      let arr_f = <FutureCategory<i32> as Category<i32, i32>>::arr(f);

      // Get the identity morphism
      let id = <FutureCategory<i32> as Category<i32, i32>>::id();

      // Compose id . f
      let id_then_f =
        <FutureCategory<i32> as Category<i32, i32>>::compose(id.clone(), arr_f.clone());

      // Compose f . id
      let f_then_id = <FutureCategory<i32> as Category<i32, i32>>::compose(arr_f.clone(), id);

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
    let x = 5;

    for f_idx in 0..INT_FUNCTIONS.len() {
      for g_idx in 0..INT_FUNCTIONS.len() {
        for h_idx in 0..INT_FUNCTIONS.len() {
          let f = INT_FUNCTIONS[f_idx];
          let g = INT_FUNCTIONS[g_idx];
          let h = INT_FUNCTIONS[h_idx];

          // Create Category::arr versions
          let arr_f = <FutureCategory<i32> as Category<i32, i32>>::arr(f);
          let arr_g = <FutureCategory<i32> as Category<i32, i32>>::arr(g);
          let arr_h = <FutureCategory<i32> as Category<i32, i32>>::arr(h);

          // Compose (f . g) . h
          let fg =
            <FutureCategory<i32> as Category<i32, i32>>::compose(arr_f.clone(), arr_g.clone());
          let fg_h = <FutureCategory<i32> as Category<i32, i32>>::compose(fg, arr_h.clone());

          // Compose f . (g . h)
          let gh = <FutureCategory<i32> as Category<i32, i32>>::compose(arr_g, arr_h);
          let f_gh = <FutureCategory<i32> as Category<i32, i32>>::compose(arr_f, gh);

          // Apply the input
          let result_fg_h = fg_h.apply(boxed_future(x)).await;
          let result_f_gh = f_gh.apply(boxed_future(x)).await;

          // Both compositions should give the same result
          assert_eq!(result_fg_h, result_f_gh);
        }
      }
    }
  }

  // Test the first combinator
  #[tokio::test]
  async fn test_first_combinator() {
    let x = 5;
    let c = 10;

    for f_idx in 0..INT_FUNCTIONS.len() {
      let f = INT_FUNCTIONS[f_idx];

      // Create the arr version
      let arr_f = <FutureCategory<i32> as Category<i32, i32>>::arr(f);

      // Apply first to get a function on pairs
      let first_f = <FutureCategory<i32> as Category<i32, i32>>::first(arr_f);

      // Apply the first combinator
      let result = first_f.apply(boxed_future((x, c))).await;

      // The expected result is (f(x), c)
      let expected = (f(&x), c);

      // Results should match
      assert_eq!(result, expected);
    }
  }

  // Test the second combinator
  #[tokio::test]
  async fn test_second_combinator() {
    let x = 5;
    let c = 10;

    for f_idx in 0..INT_FUNCTIONS.len() {
      let f = INT_FUNCTIONS[f_idx];

      // Create the arr version
      let arr_f = <FutureCategory<i32> as Category<i32, i32>>::arr(f);

      // Apply second to get a function on pairs
      let second_f = <FutureCategory<i32> as Category<i32, i32>>::second(arr_f);

      // Apply the second combinator
      let result = second_f.apply(boxed_future((c, x))).await;

      // The expected result is (c, f(x))
      let expected = (c, f(&x));

      // Results should match
      assert_eq!(result, expected);
    }
  }
}
