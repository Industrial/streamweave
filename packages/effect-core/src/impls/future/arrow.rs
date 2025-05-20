//! Implementation of the `Arrow` trait for `Future`.
//!
//! This module provides arrow operations for `Future` types, allowing for
//! composable asynchronous computations with support for splitting and fanout operations.

use crate::impls::future::category::FutureCategory;
use crate::traits::arrow::Arrow;
use crate::types::threadsafe::CloneableThreadSafe;

impl<T: CloneableThreadSafe + Send + 'static> Arrow<T, T> for FutureCategory<T> {
  fn arrow<A: CloneableThreadSafe, B: CloneableThreadSafe, F>(f: F) -> Self::Morphism<A, B>
  where
    F: Fn(A) -> B + CloneableThreadSafe,
  {
    let f = std::sync::Arc::new(f);
    Self::Morphism::new(move |future| {
      let f = f.clone();
      Box::pin(async move {
        let a = future.await;
        f(a)
      })
    })
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
    Self::Morphism::new(move |future| {
      let f = f.clone();
      let g = g.clone();
      Box::pin(async move {
        let (t, a) = future.await;
        
        // Prepare future for t
        let t_fut = Box::new(std::future::ready(t));
        let t_result = f.apply(t_fut).await;
        
        // Prepare future for a
        let a_fut = Box::new(std::future::ready(a));
        let b_result = g.apply(a_fut).await;
        
        (t_result, b_result)
      })
    })
  }

  fn fanout<C: CloneableThreadSafe>(
    f: Self::Morphism<T, T>,
    g: Self::Morphism<T, C>,
  ) -> Self::Morphism<T, (T, C)> {
    Self::Morphism::new(move |future| {
      let f = f.clone();
      let g = g.clone();
      Box::pin(async move {
        let t: T = future.await;
        
        // Clone t for each computation
        let t1_fut = Box::new(std::future::ready(t.clone()));
        let t2_fut = Box::new(std::future::ready(t));
        
        // Apply the functions
        let t_result = f.apply(t1_fut).await;
        let c_result = g.apply(t2_fut).await;
        
        (t_result, c_result)
      })
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::impls::future::category::FutureCategory;
  use crate::traits::category::Category;
  use std::future::Future;
  
  // Helper to convert a value to a boxed future
  fn boxed_future<T: Send + 'static>(value: T) -> Box<dyn Future<Output = T> + Send + Unpin> {
    Box::new(std::future::ready(value))
  }
  
  // Define test functions
  const INT_FUNCTIONS: &[fn(i64) -> i64] = &[
    |x| x.saturating_add(1),
    |x| x.saturating_mul(2),
    |x| x.saturating_sub(1),
    |x| if x != 0 { x / 2 } else { 0 },
    |x| x.saturating_mul(x),
  ];
  
  #[tokio::test]
  async fn test_arrow_creation() {
    let x = 42i64;
    
    for f_idx in 0..INT_FUNCTIONS.len() {
      let f = INT_FUNCTIONS[f_idx];
      
      // Create an arrow from a function
      let arrow_f = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(f);
      
      // Apply the arrow to a future
      let result = arrow_f.apply(boxed_future(x)).await;
      
      // Verify the result
      assert_eq!(result, f(x));
    }
  }
  
  #[tokio::test]
  async fn test_split() {
    let t = 5i64;
    let a = 10i64;
    
    let add_one = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(|x: i64| x + 1);
    let double = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(|x: i64| x * 2);
    
    let split = <FutureCategory<i64> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(add_one, double);
    
    let result = split.apply(boxed_future((t, a))).await;
    assert_eq!(result, (6, 20)); // (5+1, 10*2)
  }
  
  #[tokio::test]
  async fn test_fanout() {
    let t = 5i64;
    
    let add_one = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(|x: i64| x + 1);
    let double = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(|x: i64| x * 2);
    
    let fanout = <FutureCategory<i64> as Arrow<i64, i64>>::fanout(add_one, double);
    
    let result = fanout.apply(boxed_future(t)).await;
    assert_eq!(result, (6, 10)); // (5+1, 5*2)
  }
  
  #[tokio::test]
  async fn test_arrow_laws() {
    // Test the arrow law: arrow(id) = id
    let x = 42i64;
    let identity = |x: i64| x;
    
    let arrow_id = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(identity);
    let category_id = <FutureCategory<i64> as Category<i64, i64>>::id();
    
    let result_arrow = arrow_id.apply(boxed_future(x)).await;
    let result_category = category_id.apply(boxed_future(x)).await;
    
    assert_eq!(result_arrow, result_category);
    
    // Test law: arrow(f >>> g) = arrow(f) >>> arrow(g)
    let f = |x: i64| x + 1;
    let g = |x: i64| x * 2;
    let fg = move |x: i64| g(f(x));
    
    let arrow_f = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(f);
    let arrow_g = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(g);
    let arrow_fg = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(fg);
    
    let composed = <FutureCategory<i64> as Category<i64, i64>>::compose(arrow_f, arrow_g);
    
    let result_fg = arrow_fg.apply(boxed_future(x)).await;
    let result_composed = composed.apply(boxed_future(x)).await;
    
    assert_eq!(result_fg, result_composed);
  }
  
  #[tokio::test]
  async fn test_split_law() {
    // Test the arrow law: split(arr(f), arr(g)) = arr(\(x,y) -> (f x, g y))
    let x = 5i64;
    let y = 10i64;
    
    let f = |x: i64| x + 1;
    let g = |y: i64| y * 2;
    
    let arrow_f = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(f);
    let arrow_g = <FutureCategory<i64> as Arrow<i64, i64>>::arrow(g);
    
    let split = <FutureCategory<i64> as Arrow<i64, i64>>::split::<i64, i64, (), ()>(arrow_f, arrow_g);
    
    let direct = <FutureCategory<(i64, i64)> as Arrow<(i64, i64), (i64, i64)>>::arrow(move |(x, y)| (f(x), g(y)));
    
    let result_split = split.apply(boxed_future((x, y))).await;
    let result_direct = direct.apply(boxed_future((x, y))).await;
    
    assert_eq!(result_split, result_direct);
  }
} 